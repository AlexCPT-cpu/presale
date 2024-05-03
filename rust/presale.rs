use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
    solana_program::clock::UnixTimestamp,
    sysvar::clock,
};
use spl_token::{
    instruction::{transfer as token_transfer_instruction},
    state::Account as TokenAccount,
};

entrypoint!(process_instruction);

struct CandyMachine {
    authority: Pubkey,
    token_account: Pubkey,
    total_tokens: u64,
    token_price: u64,
    sold_tokens: u64,
    release_percentage: u8, // Percentage released on TGE
    vesting_period_months: u8,
    last_release_timestamp: UnixTimestamp,
    buyer_purchases: Vec<(Pubkey, u64, u64)>, // Store each buyer's purchase amount and claimable tokens
}

impl CandyMachine {
    fn new(authority: Pubkey, token_account: Pubkey, total_tokens: u64, token_price: u64, release_percentage: u8, vesting_period_months: u8) -> Self {
        Self {
            authority,
            token_account,
            total_tokens,
            token_price,
            sold_tokens: 0,
            release_percentage,
            vesting_period_months,
            last_release_timestamp: 0, // Initial value
            buyer_purchases: Vec::new(), // Initialize empty vector
        }
    }

    fn purchase_tokens(&mut self, buyer_account: &AccountInfo, amount: u64) -> ProgramResult {
        let amount_to_pay = amount * self.token_price;
        let buyer_lamports = buyer_account.lamports();
        
        if buyer_lamports < amount_to_pay {
            return Err(ProgramError::InsufficientFunds);
        }

        // Check if tokens are available for sale
        let remaining_tokens = self.total_tokens - self.sold_tokens;
        let tokens_to_sell = remaining_tokens.min(amount);

        // Calculate and update sold tokens
        let tokens_sold = tokens_to_sell * self.token_price;
        self.sold_tokens += tokens_sold;

        // Calculate claimable tokens (20% initially)
        let claimable_tokens = tokens_sold * 20 / 100;

        // Check if the buyer has already made a purchase
        let mut buyer_found = false;
        for (_, existing_amount, existing_claimable) in &mut self.buyer_purchases {
            if buyer_account.key == existing_amount {
                // If buyer found, update existing purchase entry
                *existing_amount += amount;
                *existing_claimable += claimable_tokens;
                buyer_found = true;
                break;
            }
        }

        // If buyer not found, create a new purchase entry
        if !buyer_found {
            self.buyer_purchases.push((*buyer_account.key, amount, claimable_tokens));
        }

        Ok(())
    }


    fn claim_tokens(&mut self, beneficiary_account: &AccountInfo) -> ProgramResult {
        let mut total_claimable_tokens = 0;
        let current_timestamp = clock::get()?.unix_timestamp;

        // Calculate total claimable tokens for the beneficiary
        for (_, _, claimable_tokens) in &self.buyer_purchases {
            total_claimable_tokens += claimable_tokens;
        }

        if total_claimable_tokens == 0 {
            return Err(ProgramError::NoVestedTokens);
        }

        // Distribute claimable tokens to beneficiary
        for (buyer_pubkey, _, claimable_tokens) in &mut self.buyer_purchases {
            let vested_amount = self.calculate_vested_amount(claimable_tokens, *buyer_pubkey, current_timestamp)?;
            if vested_amount == 0 {
                continue; // Skip if no vested tokens
            }

            // Transfer vested tokens to beneficiary
            solana_program::program::invoke(
                &spl_token::instruction::transfer(
                    &spl_token::id(),
                    self.token_account,
                    beneficiary_account.key,
                    self.authority,
                    &[],
                    vested_amount,
                )?,
                &[self.token_account.clone(), beneficiary_account.clone(), self.authority.clone()],
            )?;

            // Update claimable tokens for the buyer
            *claimable_tokens -= vested_amount;
        }

        Ok(())
    }

    fn calculate_vested_amount(&self, claimable_tokens: &u64, account: &Pubkey, current_timestamp: UnixTimestamp) -> Result<u64, ProgramError> {
        let elapsed_months = (current_timestamp - self.last_release_timestamp) / (30 * 24 * 60 * 60); // Assume 30 days per month

        // Check if vesting period is over
        if elapsed_months < 2 {
            return Ok(0); // Cliff period (no vesting)
        }

        // Calculate vested amount based on vesting schedule
        let monthly_release = *claimable_tokens * 10 / 100; // 10% released monthly
        Ok(elapsed_months * monthly_release)
    }

    fn enable_initial_claim(&mut self) -> ProgramResult {
        // Calculate total claimable tokens for the beneficiary
        let mut total_claimable_tokens = 0;
        for (_, _, claimable_tokens) in &self.buyer_purchases {
            total_claimable_tokens += claimable_tokens;
        }

        // Update last release timestamp
        self.last_release_timestamp = clock::get()?.unix_timestamp;

        Ok(())
    }

    fn get_vested_tokens(&self, account: &Pubkey) -> u64 {
        let mut vested_tokens = 0;

        // Calculate total vested tokens for the beneficiary
        for (_, _, claimable_tokens) in &self.buyer_purchases {
            vested_tokens += claimable_tokens;
        }

        vested_tokens
    }

    fn get_claimable_tokens(&self, account: &Pubkey) -> u64 {
        let mut claimable_tokens = 0;

        // Calculate total claimable tokens for the beneficiary
        for (_, _, tokens) in &self.buyer_purchases {
            claimable_tokens += tokens;
        }

        claimable_tokens
    }

    fn deposit_tokens(&mut self, admin_account: &AccountInfo, amount: u64) -> ProgramResult {
        // Transfer tokens from admin's account to the program's token account
        solana_program::program::invoke(
            &spl_token::instruction::transfer(
                &spl_token::id(),
                admin_account.key,
                &self.token_account,
                admin_account.key,
                &[],
                amount,
            )?,
            &[admin_account.clone(), self.token_account.clone()],
        )?;

        self.total_tokens += amount;

        Ok(())
    }

}


fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let candy_machine_account = next_account_info(accounts_iter)?;
    let token_account = next_account_info(accounts_iter)?;
    let authority_account = next_account_info(accounts_iter)?;

    let mut candy_machine = CandyMachine::unpack_unchecked(&candy_machine_account.data.borrow())?;

    // Parse instruction data and dispatch appropriate function
    match instruction_data {
        // Instruction to purchase tokens
        b"purchase_tokens" => {
            // Ensure instruction data contains at least 40 bytes (32 bytes for pubkey + 8 bytes for amount)
            if instruction_data.len() < 40 {
                return Err(ProgramError::InvalidInstructionData);
            }

            // Parse instruction data to extract buyer's pubkey and purchase amount
            let buyer_pubkey_bytes = &instruction_data[0..32];
            let amount_bytes = &instruction_data[32..40];
            let buyer_pubkey = Pubkey::new_from_array(*buyer_pubkey_bytes);
            let amount = u64::from_le_bytes(amount_bytes.try_into().unwrap());
    
            // Call the purchase_tokens function
            candy_machine.purchase_tokens(&buyer_pubkey, amount)?;
        },
        // Instruction to enable initial claim
        b"enable_initial_claim" => {
            // Ensure authority account is the caller
            if authority_account.key != &candy_machine.authority {
                return Err(ProgramError::InvalidAccountData);
            }

            // Call the enable_initial_claim function
            candy_machine.enable_initial_claim()?;
        },
        // Instruction to claim tokens
        b"claim_tokens" => {
            // Ensure beneficiary account is the caller
            if authority_account.key != &candy_machine.authority {
                return Err(ProgramError::InvalidAccountData);
            }

            // Call the claim_tokens function
            candy_machine.claim_tokens(authority_account)?;
        },
        // Instruction to deposit tokens
        b"deposit_tokens" => {
            // Ensure authority account is the caller
            if authority_account.key != &candy_machine.authority {
                return Err(ProgramError::InvalidAccountData);
            }

            // Ensure token_account is a valid account
            if token_account.data_is_empty() {
                return Err(ProgramError::InvalidAccountData);
            }

            let amount_bytes = &instruction_data[0..8];
            let amount = u64::from_le_bytes(amount_bytes.try_into().unwrap());

            // Call the deposit_tokens function
            candy_machine.deposit_tokens(token_account, amount)?;
        },
        // Add more cases for other instructions if needed
        _ => return Err(ProgramError::InvalidInstruction),
    }

    // Update candy machine account data
    candy_machine.pack_into(&mut candy_machine_account.data.borrow_mut())?;

    Ok(())
}

impl CandyMachine {
    fn pack_into<'a>(&self, dst: &'a mut [u8]) -> Result<&'a mut [u8], ProgramError> {
        // Serialization logic goes here
        // Example: Use bincode to serialize the struct into bytes
        let encoded = bincode::serialize(self)?;
        dst.copy_from_slice(&encoded);
        Ok(dst)
    }

    fn unpack_unchecked(src: &[u8]) -> Result<Self, ProgramError> {
        // Deserialization logic goes here
        // Example: Use bincode to deserialize bytes into the struct
        let decoded: Self = bincode::deserialize(src)?;
        Ok(decoded)
    }
}
