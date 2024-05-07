use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::Sysvar;
use anchor_lang::solana_program::clock::Clock;
use anchor_lang::solana_program::program::invoke_signed;
use spl_token::instruction::transfer as spl_transfer;
use spl_token::state::Account as TokenAccount;

#[program]
mod token_sale {
    use super::*;

    #[state]
    pub struct TokenSale {
        pub admin: Pubkey,
        pub token_account: Pubkey,
        pub token_price: u64,
        pub total_tokens: u64,
        pub sold_tokens: u64,
        pub claimable_percentage: u8,
        pub vesting_period_months: u8,
        pub last_release_timestamp: i64,
        pub enable_claiming: bool,
        pub buyers: Vec<(Pubkey, u64, u64)>,
    }

    impl TokenSale {
        pub fn new(
            ctx: Context<NewTokenSale>,
            token_account: AccountInfo<'info>,
            token_price: u64,
            total_tokens: u64,
            claimable_percentage: u8,
            vesting_period_months: u8,
        ) -> Result<Self, ProgramError> {
            let sale = Self {
                admin: *ctx.accounts.admin.key(),
                token_account: *token_account.key(),
                token_price,
                total_tokens,
                sold_tokens: 0,
                claimable_percentage,
                vesting_period_months,
                last_release_timestamp: 0,
                enable_claiming: false,
                buyers: Vec::new(),
            };
            Ok(sale)
        }

        pub fn buy_tokens(
            &mut self,
            ctx: Context<BuyTokens>,
            amount: u64,
        ) -> ProgramResult {
            let amount_to_pay = amount * self.token_price;
            if amount_to_pay > ctx.accounts.buyer.lamports() {
                return Err(ProgramError::Custom(1)); // Insufficient funds
            }

            let remaining_tokens = self.total_tokens - self.sold_tokens;
            let tokens_to_sell = remaining_tokens.min(amount);

            let tokens_sold = tokens_to_sell * self.token_price;
            self.sold_tokens += tokens_sold;

            let claimable_tokens = tokens_sold * self.claimable_percentage as u64 / 100;

            let mut buyer_found = false;
            for (existing_buyer_account, existing_amount, existing_claimable) in &mut self.buyers {
                if &ctx.accounts.buyer.key() == existing_buyer_account {
                    *existing_amount += amount;
                    *existing_claimable += claimable_tokens;
                    buyer_found = true;
                    break;
                }
            }

            if !buyer_found {
                self.buyers.push((ctx.accounts.buyer.key(), amount, claimable_tokens));
            }

            ctx.accounts.buyer.try_account_ref_mut()?.lamports -= amount_to_pay;
            Ok(())
        }

        pub fn enable_claiming(
            &mut self,
            ctx: Context<EnableClaiming>,
        ) -> ProgramResult {
            if *ctx.accounts.admin.key() != self.admin {
                return Err(ProgramError::InvalidAccountData);
            }

            self.enable_claiming = true;
            self.last_release_timestamp = Clock::get()?.unix_timestamp;
            Ok(())
        }

        pub fn claim_tokens(
            &mut self,
            ctx: Context<ClaimTokens>,
        ) -> ProgramResult {
            if !self.enable_claiming {
                return Err(ProgramError::Custom(2)); // Claiming not enabled yet
            }

            let current_timestamp = Clock::get()?.unix_timestamp;
            let mut total_claimable_tokens = 0;

            for (_, _, claimable_tokens) in &self.buyers {
                total_claimable_tokens += claimable_tokens;
            }

            if total_claimable_tokens == 0 {
                return Err(ProgramError::Custom(3)); // No vested tokens
            }

            for (buyer_pubkey, _, claimable_tokens) in &mut self.buyers {
                let vested_amount = self.calculate_vested_amount(*claimable_tokens, *buyer_pubkey, current_timestamp)?;

                let transfer_ix = spl_transfer(
                    &ctx.accounts.token_program.key(),
                    &self.token_account,
                    ctx.accounts.beneficiary.key(),
                    ctx.accounts.admin.key(),
                    &[],
                    vested_amount,
                )?;

                let seeds = &[&self.admin.to_bytes(), &[buyer_pubkey], &[self.token_account]];

                invoke_signed(
                    &transfer_ix,
                    &[
                        self.token_account.clone(),
                        ctx.accounts.beneficiary.clone(),
                        ctx.accounts.admin.clone(),
                        ctx.accounts.token_program.clone(),
                    ],
                    &[seeds],
                )?;

                *claimable_tokens -= vested_amount;
            }

            Ok(())
        }

        fn calculate_vested_amount(
            &self,
            claimable_tokens: u64,
            _account: Pubkey,
            current_timestamp: i64,
        ) -> Result<u64, ProgramError> {
            let elapsed_months = (current_timestamp - self.last_release_timestamp) / (30 * 24 * 60 * 60);

            if elapsed_months < 2 {
                return Ok(0); // Cliff period (no vesting)
            }

            let monthly_release = claimable_tokens * 10 / 100; // 10% released monthly
            Ok(elapsed_months as u64 * monthly_release)
        }

        pub fn change_token_price(
            &mut self,
            ctx: Context<ChangeTokenPrice>,
            new_price: u64,
        ) -> ProgramResult {
            if *ctx.accounts.admin.key() != self.admin {
                return Err(ProgramError::InvalidAccountData);
            }

            self.token_price = new_price;
            Ok(())
        }

        pub fn deposit_tokens(
            &mut self,
            ctx: Context<DepositTokens>,
            amount: u64,
        ) -> ProgramResult {
            if *ctx.accounts.admin.key() != self.admin {
                return Err(ProgramError::InvalidAccountData);
            }

            let token_account: &mut TokenAccount = ctx.accounts.token_account.load()?;
            token_account.amount += amount;
            Ok(())
        }

        pub fn withdraw_funds(
            &mut self,
            ctx: Context<WithdrawFunds>,
            recipient: AccountInfo<'info>,
        ) -> ProgramResult {
            if *ctx.accounts.admin.key() != self.admin {
                return Err(ProgramError::InvalidAccountData);
            }

            let balance = ctx.accounts.admin.lamports();
            **ctx.accounts.admin.try_borrow_mut_lamports()? -= balance;
            **recipient.try_borrow_mut_lamports()? += balance;
            Ok(())
        }

        // Read functions...

        pub fn get_token_sale_data(&self) -> TokenSaleData {
            TokenSaleData {
                admin: self.admin,
                token_account: self.token_account,
                token_price: self.token_price,
                total_tokens: self.total_tokens,
                sold_tokens: self.sold_tokens,
                claimable_percentage: self.claimable_percentage,
                vesting_period_months: self.vesting_period_months,
                last_release_timestamp: self.last_release_timestamp,
                enable_claiming: self.enable_claiming,
            }
        }

        pub fn get_buyer_info(&self, buyer_pubkey: Pubkey) -> Option<BuyerInfo> {
            for (pubkey, amount, claimable) in &self.buyers {
                if *pubkey == buyer_pubkey {
                    return Some(BuyerInfo {
                        pubkey: *pubkey,
                        amount: *amount,
                        claimable: *claimable,
                    });
                }
            }
            None
        }

        pub fn get_total_claimable_tokens(&self) -> u64 {
            let mut total_claimable_tokens = 0;
            for (_, _, claimable_tokens) in &self.buyers {
                total_claimable_tokens += claimable_tokens;
            }
            total_claimable_tokens
        }

        pub fn get_total_vested_tokens(&self, buyer_pubkey: Pubkey) -> Option<u64> {
            for (pubkey, _, claimable) in &self.buyers {
                if *pubkey == buyer_pubkey {
                    return Some(*claimable);
                }
            }
            None
        }
    }

    #[derive(Accounts)]
    pub struct NewTokenSale<'info> {
        #[account(signer)]
        pub admin: AccountInfo<'info>,
        pub token_account: AccountInfo<'info>,
        pub system_program: AccountInfo<'info>,
    }

    #[derive(Accounts)]
    pub struct BuyTokens<'info> {
        #[account(signer)]
        pub buyer: AccountInfo<'info>,
        #[account(mut)]
        pub token_sale: AccountInfo<'info>,
        #[account(mut)]
        pub token_program: AccountInfo<'info>,
        pub system_program: AccountInfo<'info>,
    }

    #[derive(Accounts)]
    pub struct EnableClaiming<'info> {
        #[account(signer)]
        pub admin: AccountInfo<'info>,
        #[account(mut)]
        pub token_sale: AccountInfo<'info>,
        pub system_program: AccountInfo<'info>,
    }

    #[derive(Accounts)]
    pub struct ClaimTokens<'info> {
        #[account(signer)]
        pub beneficiary: AccountInfo<'info>,
        #[account(mut)]
        pub token_sale: AccountInfo<'info>,
        #[account(mut)]
        pub token_program: AccountInfo<'info>,
        pub system_program: AccountInfo<'info>,
    }

    #[derive(Accounts)]
    pub struct ChangeTokenPrice<'info> {
        #[account(signer)]
        pub admin: AccountInfo<'info>,
        #[account(mut)]
        pub token_sale: AccountInfo<'info>,
    }

    #[derive(Accounts)]
    pub struct DepositTokens<'info> {
        #[account(signer)]
        pub admin: AccountInfo<'info>,
        #[account(mut)]
        pub token_account: AccountInfo<'info>,
        #[account(mut)]
        pub token_sale: AccountInfo<'info>,
    }

    #[derive(Accounts)]
    pub struct WithdrawFunds<'info> {
        #[account(signer)]
        pub admin: AccountInfo<'info>,
        pub recipient: AccountInfo<'info>,
    }

    #[account]
    pub struct TokenSaleData {
        pub admin: Pubkey,
        pub token_account: Pubkey,
        pub token_price: u64,
        pub total_tokens: u64,
        pub sold_tokens: u64,
        pub claimable_percentage: u8,
        pub vesting_period_months: u8,
        pub last_release_timestamp: i64,
        pub enable_claiming: bool,
    }

    #[account]
    pub struct BuyerInfo {
        pub pubkey: Pubkey,
        pub amount: u64,
        pub claimable: u64,
    }
}
