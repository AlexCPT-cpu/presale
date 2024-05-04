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

struct VestingContract {
    admin: Pubkey,
    token_account: Pubkey,
    total_tokens: u64,
    cliff_period_months: u8,
    monthly_release_percentage: u8,
    last_claim_timestamp: UnixTimestamp,
}

impl VestingContract {
    fn new(admin: Pubkey, token_account: Pubkey, total_tokens: u64, cliff_period_months: u8, monthly_release_percentage: u8) -> Self {
        Self {
            admin,
            token_account,
            total_tokens,
            cliff_period_months,
            monthly_release_percentage,
            last_claim_timestamp: 0, // Initial value
        }
    }

    fn deposit_tokens(&mut self, admin_account: &AccountInfo, amount: u64) -> ProgramResult {
        // Ensure admin is the caller
        if *admin_account.key != self.admin {
            return Err(ProgramError::InvalidAccountData);
        }

        // Transfer tokens from admin's account to the vesting contract's token account
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


    fn claim_tokens(&mut self, beneficiary_account: &AccountInfo, admin_account: &AccountInfo) -> ProgramResult {
        // Verify that the caller is the contract owner (admin)
        if *admin_account.key != self.admin {
            return Err(ProgramError::InvalidAccountData);
        }
    
        let current_timestamp = clock::get()?.unix_timestamp;
    
        // Check if cliff period is over
        let elapsed_months = (current_timestamp - self.last_claim_timestamp) / (30 * 24 * 60 * 60); // Assume 30 days per month
        if elapsed_months < self.cliff_period_months.into() {
            return Err(ProgramError::InvalidAccountData);
        }
    
        // Calculate vested amount based on vesting schedule
        let total_vested_tokens = self.total_tokens * self.monthly_release_percentage as u64 / 100;
        let vested_tokens = total_vested_tokens * elapsed_months;
    
        // Transfer vested tokens from vesting contract's token account to beneficiary's token account
        solana_program::program::invoke(
            &spl_token::instruction::transfer(
                &spl_token::id(),
                self.token_account,
                beneficiary_account.key,
                self.admin,
                &[],
                vested_tokens,
            )?,
            &[self.token_account.clone(), beneficiary_account.clone(), self.admin.clone()],
        )?;
    
        // Update last claim timestamp
        self.last_claim_timestamp = current_timestamp;
    
        Ok(())
    }
    
    fn get_vested_tokens(&self) -> u64 {
        let current_timestamp = clock::get().unwrap().unix_timestamp;
        if current_timestamp < self.cliff_timestamp {
            return 0;
        }

        let elapsed_months = (current_timestamp - self.last_release_timestamp) / (30 * 24 * 60 * 60); // Assume 30 days per month
        if elapsed_months == 0 {
            return 0;
        }

        self.total_tokens * (self.release_percentage as u64) * elapsed_months / 100
    }

    fn get_claimable_tokens(&self) -> u64 {
        let current_timestamp = clock::get().unwrap().unix_timestamp;
        if current_timestamp < self.cliff_timestamp {
            return 0;
        }

        let elapsed_months = (current_timestamp - self.last_release_timestamp) / (30 * 24 * 60 * 60); // Assume 30 days per month
        if elapsed_months == 0 {
            return 0;
        }

        self.total_tokens * (self.release_percentage as u64) * elapsed_months / 100
    }
}

fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let vesting_contract_account = next_account_info(accounts_iter)?;
    let token_account = next_account_info(accounts_iter)?;
    let admin_account = next_account_info(accounts_iter)?;

    let mut vesting_contract = VestingContract::unpack_unchecked(&vesting_contract_account.data.borrow())?;

    // Parse instruction data and dispatch appropriate function
    match instruction_data {
        // Instruction to deposit tokens
        b"deposit_tokens" => {
            // Ensure instruction data contains 8 bytes for the amount
            if instruction_data.len() != 8 {
                return Err(ProgramError::InvalidInstructionData);
            }

            // Parse instruction data to extract the amount
            let amount = u64::from_le_bytes(instruction_data.try_into().unwrap());

            // Call the deposit_tokens function
            vesting_contract.deposit_tokens(admin_account, amount)?;
        },
        // Instruction to claim vested tokens
        b"claim_tokens" => {
            // Call the claim_tokens function
            vesting_contract.claim_tokens(vesting_contract_account)?;
        },
        _ => return Err(ProgramError::InvalidInstruction),
    }

    // Update vesting contract account data
    vesting_contract.pack_into(&mut vesting_contract_account.data.borrow_mut())?;

    Ok(())
}

impl VestingContract {
    fn pack_into<'a>(&self, dst: &'a mut [u8]) -> Result<&'a mut [u8], ProgramError> {
        let encoded = bincode::serialize(self)?;
        dst.copy_from_slice(&encoded);
        Ok(dst)
    }

    fn unpack_unchecked(src: &[u8]) -> Result<Self, ProgramError> {
        let decoded: Self = bincode::deserialize(src)?;
        Ok(decoded)
    }
}