use anchor_lang::prelude::*;
use anchor_lang::system_program::{create_account, CreateAccount};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenInterface},
};
use spl_transfer_hook_interface::instruction::TransferHookInstruction;

use spl_tlv_account_resolution::{
    account::ExtraAccountMeta, seeds::Seed, state::ExtraAccountMetaList,
};
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

declare_id!("6y4mdSAVEavdGsjVoMnEd611V7GgnTMCHLtoyfPQGHJJ");

#[program]
pub mod transfer_hook {
    use super::*;

    pub fn initialize_extra_account(ctx: Context<InitializeExtraAccountMeta>) -> Result<()> {
        // 这是我们需要的额外账户的向量。在我们的例子中
        // 只有一个账户 - 鲸鱼详细信息账户。
        let account_metas = vec![ExtraAccountMeta::new_with_seeds(
            &[Seed::Literal {
                bytes: "whale_account".as_bytes().to_vec(),
            }],
            false,
            true,
        )?];

        // 计算账户大小和租金
        let account_size = ExtraAccountMetaList::size_of(account_metas.len())? as u64;
        let lamports = Rent::get()?.minimum_balance(account_size as usize);

        // 获取mint账户公钥。
        let mint = ctx.accounts.mint.key();

        // ExtraAccountMetaList PDA 的种子。
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"extra-account-metas",
            &mint.as_ref(),
            &[ctx.bumps.extra_account_meta_list],
        ]];

        // 创建 ExtraAccountMetaList 账户
        create_account(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                CreateAccount {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.extra_account_meta_list.to_account_info(),
                },
            )
            .with_signer(signer_seeds),
            lamports,
            account_size,
            ctx.program_id,
        )?;

        // 使用额外账户初始化 ExtraAccountMetaList 账户
        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?,
            &account_metas,
        )?;

        Ok(())
    }

    pub fn transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        msg!(&format!("Transfer hook fired for an amount of {}", amount));

        if amount >= 1000 * (u64::pow(10, ctx.accounts.mint.decimals as u32)) {
            // 我们有一个鲸鱼！
            ctx.accounts.latest_whale_account.whale_address = ctx.accounts.owner.key();
            ctx.accounts.latest_whale_account.transfer_amount = amount;

            emit!(WhaleTransferEvent {
                whale_address: ctx.accounts.owner.key(),
                transfer_amount: amount
            });
        }

        Ok(())
    }
}

pub fn fallback<'info>(
    program_id: &Pubkey,
    accounts: &'info [AccountInfo<'info>],
    data: &[u8],
) -> Result<()> {
    let instruction = TransferHookInstruction::unpack(data)?;

    // 匹配指令识别符以执行 transfer hook 接口指令
    // token2022 程序在代币转移时调用此指令
    match instruction {
        TransferHookInstruction::Execute { amount } => {
            let amount_bytes = amount.to_le_bytes();

            // 在我们的程序中调用自定义 transfer hook 指令
            __private::__global::transfer_hook(program_id, accounts, &amount_bytes)
        }
        _ => return Err(ProgramError::InvalidInstructionData.into()),
    }
}

#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(token::mint = mint, token::authority = owner)]
    pub source_token: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(token::mint = mint)]
    pub destination_token: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: source token account owner,   
    /// can be SystemAccount or PDA owned by another program  
    pub owner: UncheckedAccount<'info>,
    /// CHECK: ExtraAccountMetaList Account,
    #[account(seeds = [b"extra-account-metas", mint.key().as_ref()],bump)]
    pub extra_account_meta_list: UncheckedAccount<'info>,
    #[account(mut, seeds=[b"whale_account"], bump)]
    pub latest_whale_account: Account<'info, WhaleAccount>,
}

#[derive(Accounts)]
pub struct InitializeExtraAccountMeta<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: ExtraAccountMetaList Account, must use these exact seeds
    #[account(mut, seeds=[b"extra-account-metas", mint.key().as_ref()], bump)]
    pub extra_account_meta_list: AccountInfo<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(init, seeds=[b"whale_account"], bump, payer=payer, space=8+32+8)]
    pub latest_whale_account: Account<'info, WhaleAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct WhaleAccount {
    pub whale_address: Pubkey,
    pub transfer_amount: u64,
}

#[event]
pub struct WhaleTransferEvent {
    pub whale_address: Pubkey,
    pub transfer_amount: u64,
}
