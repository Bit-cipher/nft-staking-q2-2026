use anchor_lang::prelude::*;
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::{AddPluginV1CpiBuilder, UpdatePluginV1CpiBuilder},
    types::{Attribute, Attributes, Plugin, PluginAuthority, PluginType, UpdateAuthority},
    ID as MPL_CORE_ID,
};

use crate::error::ErrorCode;
use crate::state::Config;

#[derive(Accounts)]
pub struct Stake<'info> {
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
        constraint = asset.update_authority == UpdateAuthority::Collection(collection.key()) @ ErrorCode::InvalidUpdateAuthority,
    )]
    pub asset: Account<'info, BaseAssetV1>,
    #[account(
        mut,
        has_one = update_authority @ ErrorCode::InvalidUpdateAuthority,
    )]
    pub collection: Account<'info, BaseCollectionV1>,
    /// CHECK: PDA signs MPL Core updates; it is derived and constrained by seeds.
    #[account(
        seeds = [b"update_authority", collection.key().as_ref()],
        bump
    )]
    pub update_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: Constrained to the MPL Core program id.
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<Stake>) -> Result<()> {
    let attributes_fetched = fetch_plugin::<BaseAssetV1, Attributes>(
        &ctx.accounts.asset.to_account_info(),
        PluginType::Attributes,
    )
    .ok()
    .map(|(_, attributes, _)| attributes);

    let mut attribute_list = Vec::new();

    if let Some(attributes) = &attributes_fetched {
        for attribute in &attributes.attribute_list {
            if attribute.key == "staked" {
                require!(attribute.value == "false", ErrorCode::AlreadyStaked);
            } else if attribute.key != "staked_at" {
                attribute_list.push(attribute.clone());
            }
        }
    }

    attribute_list.push(Attribute {
        key: "staked".to_string(),
        value: "true".to_string(),
    });
    attribute_list.push(Attribute {
        key: "staked_at".to_string(),
        value: Clock::get()?.unix_timestamp.to_string(),
    });

    let collection_key = ctx.accounts.collection.key();
    let update_authority_bump = [ctx.bumps.update_authority];
    let signer_seeds: &[&[u8]] = &[
        b"update_authority",
        collection_key.as_ref(),
        update_authority_bump.as_ref(),
    ];
    let signer_seeds = &[signer_seeds];

    let plugin = Plugin::Attributes(Attributes { attribute_list });

    if attributes_fetched.is_some() {
        UpdatePluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
            .asset(&ctx.accounts.asset.to_account_info())
            .collection(Some(&ctx.accounts.collection.to_account_info()))
            .payer(&ctx.accounts.owner.to_account_info())
            .authority(Some(&ctx.accounts.update_authority.to_account_info()))
            .system_program(&ctx.accounts.system_program.to_account_info())
            .plugin(plugin)
            .invoke_signed(signer_seeds)?;
    } else {
        AddPluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
            .asset(&ctx.accounts.asset.to_account_info())
            .collection(Some(&ctx.accounts.collection.to_account_info()))
            .payer(&ctx.accounts.owner.to_account_info())
            .authority(Some(&ctx.accounts.update_authority.to_account_info()))
            .system_program(&ctx.accounts.system_program.to_account_info())
            .plugin(plugin)
            .init_authority(PluginAuthority::UpdateAuthority)
            .invoke_signed(signer_seeds)?;
    }

    Ok(())
}