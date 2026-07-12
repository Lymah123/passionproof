use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;
use anchor_lang::solana_program::system_instruction;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::{
        spl_token_2022::{
            extension::{metadata_pointer, ExtensionType, StateWithExtensions},
            instruction as token_instruction,
            state::Mint as Token2022MintState,
        },
        Token2022,
    },
    token_interface::{Mint, TokenAccount},
};
use spl_token_metadata_interface::{instruction as metadata_instruction, state::TokenMetadata};

declare_id!("HSSLcVQmCdCo8qt9UBMSAL9vbqpxYLkCLHoz74dgrBE1");

// PassionProof: a Solana soulbound badge for consistent, meaningful contribution.
//
// Every badge is a Token-2022 mint with:
//   1. NonTransferable extension  -> the token can never leave the recipient's wallet
//   2. MetadataPointer extension  -> metadata lives directly on the mint account
//   3. TokenMetadata + additional fields -> a verifiable, on-chain "reason" record
//
// Two-instruction flow:
//   create_badge_mint -> builds the mint + writes metadata (name/category/reason/recipient/date)
//   mint_badge        -> mints exactly 1 unit to the recipient's ATA (inherits NonTransferable)

#[program]
pub mod passionproof {
    use super::*;

    pub fn create_badge_mint(
        ctx: Context<CreateBadgeMint>,
        name: String,
        symbol: String,
        uri: String,
        category: String,
        awarded_for: String,
        recipient_name: String,
    ) -> Result<()> {
        require!(name.len() <= 32, PassionProofError::FieldTooLong);
        require!(symbol.len() <= 10, PassionProofError::FieldTooLong);
        require!(uri.len() <= 200, PassionProofError::FieldTooLong);

        let mint = &ctx.accounts.mint;
        let payer = &ctx.accounts.payer;
        let token_program = &ctx.accounts.token_program;
        let system_program = &ctx.accounts.system_program;

        // --- 1. Size + rent for base mint with NonTransferable + MetadataPointer extensions ---
        let extension_types = vec![
            ExtensionType::NonTransferable,
            ExtensionType::MetadataPointer,
        ];
        let base_space =
            ExtensionType::try_calculate_account_len::<Token2022MintState>(&extension_types)
                .map_err(|_| error!(PassionProofError::ExtensionSizeError))?;
        let base_rent = Rent::get()?.minimum_balance(base_space);

        invoke(
            &system_instruction::create_account(
                payer.key,
                mint.key,
                base_rent,
                base_space as u64,
                token_program.key,
            ),
            &[
                payer.to_account_info(),
                mint.to_account_info(),
                system_program.to_account_info(),
            ],
        )?;

        // --- 2. Initialize NonTransferable extension (must happen before InitializeMint) ---
        invoke(
            &token_instruction::initialize_non_transferable_mint(token_program.key, mint.key)?,
            &[mint.to_account_info()],
        )?;

        // --- 3. Initialize MetadataPointer, pointing at the mint account itself ---
        invoke(
            &metadata_pointer::instruction::initialize(
                token_program.key,
                mint.key,
                Some(ctx.accounts.mint_authority.key()),
                Some(mint.key()),
            )?,
            &[mint.to_account_info()],
        )?;

        // --- 4. Initialize the mint itself (0 decimals -> a single non-fungible badge) ---
        invoke(
            &token_instruction::initialize_mint2(
                token_program.key,
                mint.key,
                &ctx.accounts.mint_authority.key(),
                Some(&ctx.accounts.mint_authority.key()),
                0,
            )?,
            &[mint.to_account_info()],
        )?;

        // --- 5. Initialize TokenMetadata (name/symbol/uri) ---
        let base_metadata = TokenMetadata {
            name: name.clone(),
            symbol: symbol.clone(),
            uri: uri.clone(),
            ..Default::default()
        };

        // Fund the extra rent metadata needs before writing it.
        let extra_len = base_metadata.tlv_size_of().unwrap_or(0)
            + 5 * (metadata_field_len("category") + category.len() + 4)
            + 5 * (metadata_field_len("awarded_for") + awarded_for.len() + 4);
        top_up_rent(payer, mint, system_program, extra_len)?;

        invoke(
            &metadata_instruction::initialize(
                token_program.key,
                mint.key,
                &ctx.accounts.mint_authority.key(),
                mint.key,
                &ctx.accounts.mint_authority.key(),
                name,
                symbol,
                uri,
            ),
            &[
                mint.to_account_info(),
                ctx.accounts.mint_authority.to_account_info(),
            ],
        )?;

        // --- 6. Write the "reason" fields: this is what makes the badge a verifiable record ---
        let fields: [(&str, String); 4] = [
            ("category", category),
            ("awarded_for", awarded_for),
            ("recipient", recipient_name),
            ("issued", Clock::get()?.unix_timestamp.to_string()),
        ];

        for (field, value) in fields {
            top_up_rent(payer, mint, system_program, field.len() + value.len() + 8)?;
            invoke(
                &metadata_instruction::update_field(
                    token_program.key,
                    mint.key,
                    &ctx.accounts.mint_authority.key(),
                    spl_token_metadata_interface::state::Field::Key(field.to_string()),
                    value,
                ),
                &[
                    mint.to_account_info(),
                    ctx.accounts.mint_authority.to_account_info(),
                ],
            )?;
        }

        Ok(())
    }

    pub fn mint_badge(ctx: Context<MintBadge>) -> Result<()> {
        // Mint exactly 1 unit into the recipient's associated token account.
        // Because the mint carries the NonTransferable extension, the resulting
        // token account is permanently locked to this recipient -- any transfer
        // instruction against it will fail on-chain.
        let cpi_accounts = anchor_spl::token_2022::MintTo {
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.recipient_token_account.to_account_info(),
            authority: ctx.accounts.mint_authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        anchor_spl::token_2022::mint_to(cpi_ctx, 1)?;

        msg!(
            "PassionProof badge minted. Soulbound to {}",
            ctx.accounts.recipient.key()
        );
        Ok(())
    }
}

// Tops up the mint account's lamports so it stays rent-exempt after a metadata
// TLV entry grows the account, then reallocs the account to fit.
fn top_up_rent<'info>(
    payer: &Signer<'info>,
    mint: &Signer<'info>,
    system_program: &Program<'info, System>,
    approx_extra_bytes: usize,
) -> Result<()> {
    let new_len = mint.data_len() + approx_extra_bytes;
    let new_rent = Rent::get()?.minimum_balance(new_len);
    let current_lamports = mint.lamports();
    if new_rent > current_lamports {
        invoke(
            &system_instruction::transfer(payer.key, mint.key, new_rent - current_lamports),
            &[
                payer.to_account_info(),
                mint.to_account_info(),
                system_program.to_account_info(),
            ],
        )?;
    }
    Ok(())
}

fn metadata_field_len(_s: &str) -> usize {
    // Small fixed overhead per TLV-encoded metadata field (type + length prefix).
    8
}

#[derive(Accounts)]
pub struct CreateBadgeMint<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The mint authority also signs as the "issuer" of the badge (e.g. a maintainer,
    /// or the recipient themselves for self-attested streaks). Kept generic on purpose.
    pub mint_authority: Signer<'info>,

    /// CHECK: created and initialized manually via CPI above with Token-2022 extensions.
    #[account(mut)]
    pub mint: Signer<'info>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintBadge<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub mint_authority: Signer<'info>,

    /// CHECK: the badge mint, created in create_badge_mint.
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,

    /// CHECK: the person this badge belongs to. Does not need to sign --
    /// anyone with authority can issue a badge to a recipient's wallet.
    pub recipient: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = recipient,
        associated_token::token_program = token_program,
    )]
    pub recipient_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[error_code]
pub enum PassionProofError {
    #[msg("Field exceeds maximum allowed length.")]
    FieldTooLong,
    #[msg("Failed to calculate account size for the requested extensions.")]
    ExtensionSizeError,
}