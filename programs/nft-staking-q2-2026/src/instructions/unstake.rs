use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{mint_to_checked, Mint, MintToChecked, TokenAccount, TokenInterface},
};
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::UpdatePluginV1CpiBuilder,
    types::{Attribute, Attributes, Plugin, PluginType, UpdateAuthority},
    ID as MPL_CORE_ID,
};

use crate::error::ErrorCode;
use crate::state::Config;

const SECONDS_PER_DAY: i64 = 86400;

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        seeds = [b"config", collection.key().as_ref()],
        bump = config.bump,
    )]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        has_one = owner @ ErrorCode::InvalidOwner,
        constraint = asset.update_authority == UpdateAuthority::Collection(collection.key()) @ErrorCode::InvalidUpdateAuthority,
    )]
    pub asset: Account<'info, BaseAssetV1>,
    #[account(
        mut,
        has_one = update_authority@ErrorCode::InvalidUpdateAuthority,
    )]
    pub collection: Account<'info, BaseCollectionV1>,

    /// CHECK: This account is not initilized and is being used for signing purpose only
    #[account(
        seeds = [b"update_authority", collection.key().as_ref()],
        bump
    )]
    pub update_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"rewards_mint", config.key().as_ref()],
        bump = config.rewards_bump,
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,
    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = rewards_mint,
        associated_token::authority = owner,
        associated_token::token_program = token_program,
    )]
    pub user_rewards_ata: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    /// CHECK: This account is constrained by address to the MPL Core program id.
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<Unstake>) -> Result<()> {
    // We start by fetching the existing attributes (if they exist)
    let attributes_fetched = fetch_plugin::<BaseAssetV1, Attributes>(
        &ctx.accounts.asset.to_account_info(),
        PluginType::Attributes,
    )
    .ok()
    .map(|(_, attributes, _)| attributes);

    require!(attributes_fetched.is_some(), ErrorCode::AssetNotStaked);

    let attributes = attributes_fetched.unwrap();

    let mut attributes_list = Vec::with_capacity(attributes.attribute_list.len());

    // Additional auxilary vairibles
    let currently_timestamp = Clock::get()?.unix_timestamp;
    let mut staked_timestamp: i64 = 0;
    let mut staked_time: i64 = 0;
    let mut is_staked = false;

    for attribute in &attributes.attribute_list {
        if attribute.key == "staked" {
            is_staked = attribute.value == "true";
        } else if attribute.key == "staked_at" {
            staked_timestamp = attribute
                .value
                .parse::<i64>()
                .map_err(|_| ErrorCode::InvalidTimestamp)?;
            // Claculate the time (in seconds) since the asset was staked
            staked_time = currently_timestamp
                .checked_sub(staked_timestamp)
                .ok_or(ErrorCode::InvalidTimestamp)?;
            // Staked time in days
            staked_time = staked_time
                .checked_div(SECONDS_PER_DAY)
                .ok_or(ErrorCode::InvalidTimestamp)?;
        } else {
            attributes_list.push(attribute.clone());
        }
    }

    require!(is_staked, ErrorCode::AssetNotStaked);
    require!(staked_timestamp > 0, ErrorCode::InvalidTimestamp);
    require!(
        staked_time >= ctx.accounts.config.freeze_period as i64,
        ErrorCode::FreezePeriodNotElapsed
    );

    //prepare signing seeds for the update authority
    let collection_key = ctx.accounts.collection.key();
    let update_authority_bump = [ctx.bumps.update_authority];
    let signer_seeds: &[&[u8]; 3] = &[
        b"update_authority",
        collection_key.as_ref(),
        update_authority_bump.as_ref(),
    ];

    // Add the staking attributes
    attributes_list.push(Attribute {
        key: "staked".to_string(),
        value: "false".to_string(),
    });

    attributes_list.push(Attribute {
        key: "staked_at".to_string(),
        value: "0".to_string(),
    });

    // If the Attributes Plugin does not exist, we add it

    UpdatePluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
        .asset(&ctx.accounts.asset.to_account_info())
        .collection(Some(&ctx.accounts.collection.to_account_info()))
        .authority(Some(&ctx.accounts.update_authority.to_account_info()))
        .payer(&ctx.accounts.owner.to_account_info())
        .system_program(&ctx.accounts.system_program.to_account_info())
        .plugin(Plugin::Attributes(Attributes {
            attribute_list: attributes_list,
        }))
        .invoke_signed(&[signer_seeds])?;

    // Calculate the amount
    let amount: u64 = (staked_time as u64)
        .checked_mul(ctx.accounts.config.rewards_bps as u64)
        .ok_or(ErrorCode::InvalidRewardsBps)?
        .checked_mul(10u64.pow(ctx.accounts.rewards_mint.decimals as u32))
        .ok_or(ErrorCode::InvalidRewardsBps)?
        .checked_div(10000u64)
        .ok_or(ErrorCode::InvalidRewardsBps)?;

    // Perpare signer sees for config PDA
    let config_bump = [ctx.accounts.config.bump];
    let config_seed: &[&[u8]; 3] = &[b"config", collection_key.as_ref(), config_bump.as_ref()];

    mint_to_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintToChecked {
                mint: ctx.accounts.rewards_mint.to_account_info(),
                to: ctx.accounts.user_rewards_ata.to_account_info(),
                authority: ctx.accounts.config.to_account_info(),
            },
            &[config_seed],
        ),
        amount,
        ctx.accounts.rewards_mint.decimals,
    )?;

    Ok(())
}
