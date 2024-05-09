use anchor_lang::prelude::*;
use anchor_lang::solana_program::entrypoint::ProgramResult;

declare_id!("CkvvUYGVEtRoD6Ky2Gs7NthwK3jhrKFkkoxJiKxKNmgU");

#[program]
pub mod smart_contracts {
    use super::*;

    // Creates a campaign
pub fn create(ctx: Context<Create>) -> ProgramResult {
        let campaign = &mut ctx.accounts.campaign;

        // Hardcoded values for the campaign
        campaign.admin = *ctx.accounts.user.key;
        // Store target amount, total tokens, and token price in lamports
        campaign.target_amount = 10 * 1_000_000; // 10 SOL converted to lamports
        campaign.amount_donated = 0;
        campaign.amount_withdrawn = 0;
        campaign.total_tokens = 100 * 1_000_000; // 100 tokens converted to lamports
        campaign.token_price = 100_000; // 0.1 SOL (100_000 lamports) per token
        // Initialize tokens_sold and sale_ongoing
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
    
    campaign.tokens_sold += tokens_to_buy;
    campaign.amount_donated += amount;

    if campaign.tokens_sold == campaign.total_tokens {
        campaign.sale_ongoing = false; // Stop the sale if all tokens are sold
    }

    Ok(())
}

    // Get the campaign
pub fn get_campaign(ctx: Context<GetCampaign>) -> Result<(Pubkey, u64, u64, u64, u64, u64, u64, bool, Vec<(Pubkey, u64)>)> {
        let campaign = &ctx.accounts.campaign;
        let _user = &ctx.accounts.user;
        // Declare local variables to store campaign field values
        let admin = campaign.admin;
        let target_amount = campaign.target_amount;
        let amount_donated = campaign.amount_donated;
        let amount_withdrawn = campaign.amount_withdrawn;
        let total_tokens = campaign.total_tokens;
        let token_price = campaign.token_price;
        let tokens_sold = campaign.tokens_sold;
        let sale_ongoing = campaign.sale_ongoing;
        let user_tokens = campaign.user_tokens.clone();
    
        // Return the values as a tuple
        Ok((
            admin,
            target_amount,
            amount_donated,
            amount_withdrawn,
            total_tokens,
            token_price,
            tokens_sold,
            sale_ongoing,
            user_tokens,
        ))
    }
    
    

// Get tokens bought for a specific user
pub fn get_tokens_bought(ctx: Context<GetTokensBought>) -> Result<u64> {
    let campaign = &ctx.accounts.campaign;
    let user = &ctx.accounts.user;

    let mut tokens_bought = 0;
    for user_token in &campaign.user_tokens {
        if user_token.0 == *user.key {
            tokens_bought = user_token.1;
            break;
        }
    }

    Ok(tokens_bought)
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
    /// CHECK:
    pub user: AccountInfo<'info>,
}


#[account]
pub struct Campaign {
    pub admin: Pubkey,
    pub target_amount: u64,
    pub amount_donated: u64,
    pub amount_withdrawn: u64,
    pub total_tokens: u64,
    pub token_price: u64,
    pub tokens_sold: u64,
    pub sale_ongoing: bool,
    pub user_tokens: Vec<(Pubkey, u64)>, // Vector to store user tokens bought
}
