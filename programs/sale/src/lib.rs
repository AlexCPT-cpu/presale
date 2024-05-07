use anchor_lang::prelude::*;
use anchor_lang::solana_program::entrypoint::ProgramResult;

declare_id!("CkvvUYGVEtRoD6Ky2Gs7NthwK3jhrKFkkoxJiKxKNmgU");

#[program]
pub mod smart_contracts {
    use super::*;

    // Creates a campaign
pub fn create(
        ctx: Context<Create>,
        name: String,
        description: String,
        target_amount: u64,
        project_url: String,
        progress_update_url: String,
        project_image_url: String,
        category: String,
        total_tokens: u64,
        token_price: u64,
    ) -> ProgramResult {
        let campaign = &mut ctx.accounts.campaign;
        campaign.name = name;
        campaign.description = description;
        campaign.target_amount = target_amount;
        campaign.project_url = project_url;
        campaign.progress_update_url = progress_update_url;
        campaign.project_image_url = project_image_url;
        campaign.category = category;
        campaign.amount_donated = 0;
        campaign.amount_withdrawn = 0;
        campaign.total_tokens = total_tokens;
        campaign.token_price = token_price;
        campaign.tokens_sold = 0;
        campaign.sale_ongoing = true; // Sale is ongoing initially
        Ok(())
    }

    // Withdraw from a campaign
pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> ProgramResult {
        let campaign = &mut ctx.accounts.campaign;
        let user = &mut ctx.accounts.user;
        // Restricts Withdrawal to campaign admin
        if campaign.admin != *user.key {
            return Err(ProgramError::IncorrectProgramId);
        }
        let rent_balance = Rent::get()?.minimum_balance(campaign.to_account_info().data_len());
        if **campaign.to_account_info().lamports.borrow() - rent_balance < amount {
            return Err(ProgramError::InsufficientFunds);
        }
        **campaign.to_account_info().try_borrow_mut_lamports()? -= amount;
        **user.to_account_info().try_borrow_mut_lamports()? += amount;
        (&mut ctx.accounts.campaign).amount_withdrawn += amount;
        Ok(())
    }

// Donate to a campaign
pub fn donate(ctx: Context<Donate>, amount: u64) -> ProgramResult {
    let mut campaign = ctx.accounts.campaign.clone();
    let user = &ctx.accounts.user;

    let tokens_left = campaign.total_tokens - campaign.tokens_sold;
    if tokens_left == 0 {
        campaign.sale_ongoing = false; // Stop the sale if all tokens are sold
        return Err(ProgramError::Custom(1001)); // Custom error code to indicate sale ended
    }

    let tokens_to_buy = amount * campaign.token_price;
    if tokens_to_buy > tokens_left {
        // Refund excess funds to the user
        let excess_funds = (tokens_to_buy - tokens_left) / campaign.token_price;
        let refund_ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.campaign.key(),
            &ctx.accounts.user.key(),
            excess_funds,
        );
        anchor_lang::solana_program::program::invoke(
            &refund_ix,
            &[ctx.accounts.campaign.to_account_info(), ctx.accounts.user.to_account_info()],
        )?;
        campaign.sale_ongoing = false; // Stop the sale if all tokens are sold
        return Err(ProgramError::Custom(1003)); // Custom error code to indicate overpayment and sale ended
    }

    campaign.tokens_sold += tokens_to_buy;

    let mut user_tokens_updated = false;
    for user_token in &mut campaign.user_tokens {
        if user_token.0 == *user.key {
            user_token.1 += tokens_to_buy; // Update user's tokens bought
            user_tokens_updated = true;
            break;
        }
    }

    if !user_tokens_updated {
        campaign.user_tokens.push((*user.key, tokens_to_buy)); // Store user's Pubkey and tokens bought
    }

    campaign.amount_donated += amount;

    let ix = anchor_lang::solana_program::system_instruction::transfer(
        &ctx.accounts.user.key(),
        &ctx.accounts.campaign.key(),
        amount,
    );
    let result = anchor_lang::solana_program::program::invoke(
        &ix,
        &[ctx.accounts.user.to_account_info(), ctx.accounts.campaign.to_account_info()],
    );
    if let Err(e) = result {
        return Err(e.into());
    }

    if campaign.tokens_sold == campaign.total_tokens {
        campaign.sale_ongoing = false; // Stop the sale if all tokens are sold
    }

    Ok(())
}

    // Get the campaign
pub fn get_campaign(ctx: Context<GetCampaign>) -> ProgramResult {
        let campaign = &ctx.accounts.campaign;
        let user = &ctx.accounts.user;
        if campaign.admin != *user.key {
            return Err(ProgramError::IncorrectProgramId);
        }
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Create<'info> {
    #[account(
        init,
        payer = user,
        space = 9000,
        seeds = [b"CROWDFUND".as_ref(), user.key().as_ref()],
        bump
    )]
    pub campaign: Account<'info, Campaign>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub campaign: Account<'info, Campaign>,
    #[account(mut)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct Donate<'info> {
    #[account(mut)]
    pub campaign: Account<'info, Campaign>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct GetCampaign<'info> {
    #[account(mut)]
    pub campaign: Account<'info, Campaign>,
    #[account(mut)]
    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct GetTokensBought<'info> {
    #[account(mut)]
    pub campaign: Account<'info, Campaign>,
    #[account(signer)]
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub user: AccountInfo<'info>,
        /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct GetPurchasers<'info> {
    #[account(mut)]
    pub campaign: Account<'info, Campaign>,
        /// CHECK: This is not dangerous because we don't read or write from this account
    pub system_program: AccountInfo<'info>,
}

#[account]
pub struct Campaign {
    pub admin: Pubkey,
    pub name: String,
    pub description: String,
    pub target_amount: u64,
    pub project_url: String,
    pub progress_update_url: String,
    pub project_image_url: String,
    pub category: String,
    pub amount_donated: u64,
    pub amount_withdrawn: u64,
    pub total_tokens: u64,
    pub token_price: u64,
    pub tokens_sold: u64,
    pub sale_ongoing: bool,
    pub user_tokens: Vec<(Pubkey, u64)>, // Vector to store user tokens bought
    pub purchasers: Vec<Pubkey>,         // Vector to store purchasers
}

// impl<'a, 'b, 'c, 'info> From<&mut Create<'info>> for ProgramResult {
//     fn from(ctx: &'a mut Create<'info>) -> ProgramResult {
//         let campaign = &mut ctx.accounts.campaign;
//         campaign.admin = *ctx.accounts.user.key;
//         campaign.name = String::default(); // Initialize other fields as needed
//         campaign.description = String::default();
//         campaign.target_amount = 0;
//         campaign.project_url = String::default();
//         campaign.progress_update_url = String::default();
//         campaign.project_image_url = String::default();
//         campaign.category = String::default();
//         campaign.amount_donated = 0;
//         campaign.amount_withdrawn = 0;
//         campaign.total_tokens = 0;
//         campaign.token_price = 0;
//         campaign.tokens_sold = 0;
//         campaign.sale_ongoing = true;
//         Ok(())
//     }
// }