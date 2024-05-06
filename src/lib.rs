
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};
use bincode;
use solana_program::program::invoke_signed;
use solana_program::transaction::Transaction;
use spl_token::instruction::transfer;
use serde::{Serialize, Deserialize};

entrypoint!(process_instruction);

#[derive(Serialize, Deserialize)]
struct CandyMachine {
    authority: Pubkey,
    token_account: Pubkey,
    total_tokens: u64,
    current_tokens: u64,
    token_price: u64,
    sold_tokens: u64,
    release_percentage: u8, // Percentage released on TGE
    vesting_period_months: u8,
    last_release_timestamp: i64, // Unix timestamp
    buyer_purchases: Vec<(Pubkey, u64, u64)>, // Store each buyer's purchase amount and claimable tokens
}

impl CandyMachine {
    fn new(
        authority: Pubkey,
        token_account: Pubkey,
        total_tokens: u64,
        token_price: u64,
        release_percentage: u8,
        vesting_period_months: u8,
    ) -> Self {
        Self {
            authority,
            token_account,
            total_tokens,
            current_tokens: 0,
            token_price,
            sold_tokens: 0,
            release_percentage,
            vesting_period_months,
            last_release_timestamp: 0, // Initial value
            buyer_purchases: Vec::new(), // Initialize empty vector
        }
    }

    fn purchase_tokens(&mut self, buyer_account: &Pubkey, amount: u64) -> ProgramResult {
        let amount_to_pay = amount * self.token_price;

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
        for (existing_buyer_account, existing_amount, existing_claimable) in &mut self.buyer_purchases {
            if buyer_account == existing_buyer_account {
                // If buyer found, update existing purchase entry
                *existing_amount += amount;
                *existing_claimable += claimable_tokens;
                buyer_found = true;
                break;
            }
        }

        // If buyer not found, create a new purchase entry
        if !buyer_found {
            self.buyer_purchases.push((*buyer_account, amount, claimable_tokens));
        }

        Ok(())
    }

    fn claim_tokens(&mut self, beneficiary_account: &AccountInfo, banks_client: &Bank,
        recent_blockhash: Hash,) -> ProgramResult {
        let mut total_claimable_tokens = 0;
        let current_timestamp = Clock::get()?.unix_timestamp;
    
        // Calculate total claimable tokens for the beneficiary
        for (_, _, claimable_tokens) in &self.buyer_purchases {
            total_claimable_tokens += claimable_tokens;
        }
    
        if total_claimable_tokens == 0 {
            return Err(ProgramError::Custom(1)); // Custom error code for no vested tokens
        }
    
        let token_program_id = spl_token::id();
    
        // Distribute claimable tokens to beneficiary
        for (buyer_pubkey, _, claimable_tokens) in &mut self.buyer_purchases {
            let vested_amount = self.calculate_vested_amount(claimable_tokens, *buyer_pubkey, current_timestamp)?;
            if vested_amount == 0 {
                continue; // Skip if no vested tokens
            }
    
            // Prepare transfer instruction
            let transfer_ix = spl_token::instruction::transfer(
                &token_program_id,
                &self.token_account,
                beneficiary_account.key,
                &self.authority, // Program's authority
                &[],
                vested_amount,
            )?;
    
            // Create and sign transaction
            let mut transaction = Transaction::new_with_payer(&[transfer_ix], Some(&self.authority)); // Program's authority signs
            let mut signer_keys = vec![buyer_pubkey]; // Include buyer's account as signer
            transaction.sign(&signer_keys, recent_blockhash);
            
            // Send transaction
            let result = banks_client.process_transaction(&transaction);
            result.map_err(|e| ProgramError::Custom(e.to_string()))?;
    
            // Update claimable tokens for the buyer
            *claimable_tokens -= vested_amount;
            self.current_tokens -= vested_amount;
        }
    
        Ok(())
    }
    

    fn calculate_vested_amount(&self, claimable_tokens: &u64, _account: Pubkey, current_timestamp: i64) -> Result<u64, ProgramError> {
        let elapsed_months = (current_timestamp - self.last_release_timestamp) / (30 * 24 * 60 * 60); // Assume 30 days per month

        // Check if vesting period is over
        if elapsed_months < 2 {
            return Ok(0); // Cliff period (no vesting)
        }

        // Calculate vested amount based on vesting schedule
        let monthly_release = *claimable_tokens * 10 / 100; // 10% released monthly
        Ok(elapsed_months as u64 * monthly_release)
    }

    fn enable_initial_claim(&mut self, authority_account: &Pubkey) -> ProgramResult {
        // Verify that the caller is the contract owner (admin)
        if *authority_account != self.authority {
            return Err(ProgramError::InvalidAccountData);
        }

        // Calculate total claimable tokens for the beneficiary
        let total_claimable_tokens = self.buyer_purchases.iter().map(|(_, _, claimable_tokens)| claimable_tokens).sum::<u64>();

        if total_claimable_tokens == 0 {
            return Err(ProgramError::Custom(1)); // Custom error code for no vested tokens
        }

        // Update last release timestamp
        self.last_release_timestamp = Clock::get()?.unix_timestamp;

        Ok(())
    }


    fn deposit_tokens(
        &mut self,
        authority_account: &AccountInfo,
        amount: u64,
    ) -> ProgramResult {
        // Verify that the caller is the contract owner (admin)
        if *authority_account.key != self.authority {
            return Err(ProgramError::InvalidAccountData);
        }

        let authority_bytes = self.authority.to_bytes();
        let token_bytes = self.token_account.to_bytes();
        let authority_signer_seeds = [&authority_bytes, &token_bytes];

        // Transfer tokens from admin's account to the program's token account
        invoke_signed(
            &transfer(
                &spl_token::id(),
                authority_account.key,
                &self.token_account,
                authority_account.key,
                &[],
                amount,
            )?,
            &[authority_account.clone(), self.token_account.clone()],
            &[&authority_signer_seeds],
        )?;

        self.current_tokens += amount;

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
    let banks_client = next_account_info(accounts_iter)?;

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
            let mut buyer_pubkey = [0u8; 32];
            buyer_pubkey.copy_from_slice(buyer_pubkey_bytes);
            let amount = u64::from_le_bytes(amount_bytes.try_into().unwrap());

            // Call the purchase_tokens function
            candy_machine.purchase_tokens(&Pubkey::new_from_array(buyer_pubkey), amount)?;
        },
        // Instruction to enable initial claim
        b"enable_initial_claim" => {
            // Ensure authority account is the caller
            if *authority_account.key != candy_machine.authority {
                return Err(ProgramError::InvalidAccountData);
            }

            // Call the enable_initial_claim function
            candy_machine.enable_initial_claim(&candy_machine.authority)?;
        },
        // Instruction to claim tokens
        b"claim_tokens" => {
            // Ensure beneficiary account is the caller
            if *authority_account.key != candy_machine.authority {
                return Err(ProgramError::InvalidAccountData);
            }

            // Call the claim_tokens function
            candy_machine.claim_tokens(authority_account, banks_client, recent_blockhash)?;
        },
        // Instruction to deposit tokens
        b"deposit_tokens" => {
            // Ensure authority account is the caller
            if *authority_account.key != candy_machine.authority {
                return Err(ProgramError::InvalidAccountData);
            }

            let amount_bytes = &instruction_data[0..8];
            let amount = u64::from_le_bytes(amount_bytes.try_into().unwrap());

            // Call the deposit_tokens function
            candy_machine.deposit_tokens(authority_account, amount)?;
        },
        // Add more cases for other instructions if needed
        _ => return Err(ProgramError::InvalidInstructionData),
    }

    // Update candy machine account data
    candy_machine.pack_into(&mut candy_machine_account.data.borrow_mut())?;

    Ok(())
}

impl CandyMachine {
    fn pack_into(&self, dst: &mut Vec<u8>) -> Result<(), ProgramError> {
        // Serialize the struct into bytes using bincode
        let encoded = bincode::serialize(self).map_err(|err| ProgramError::Custom(err.to_string()))?;
        
        // Append the serialized data to the destination vector
        dst.extend_from_slice(&encoded);
        
        Ok(())
    }

    fn unpack_unchecked(src: &[u8]) -> Result<Self, ProgramError> {
        // Deserialize bytes into the struct using bincode
        let decoded = bincode::deserialize(src).map_err(|err| ProgramError::Custom(err.to_string()))?;
        
        Ok(decoded)
    }
}