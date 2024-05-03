use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
    solana_program::clock::UnixTimestamp,
    sysvar::clock,
};
use std::convert::TryInto;

entrypoint!(process_instruction);

struct VestingContract {
    admin: Pubkey,
    beneficiary: Pubkey,
    total_tokens: u64,
    cliff_timestamp: UnixTimestamp,
    release_percentage: u8,
    last_release_timestamp: UnixTimestamp,
}

impl VestingContract {
    fn new(admin: Pubkey, beneficiary: Pubkey, total_tokens: u64, release_percentage: u8) -> Self {
        let current_timestamp = clock::get().unwrap().unix_timestamp;
        Self {
            admin,
            beneficiary,
            total_tokens,
            cliff_timestamp: current_timestamp + 2 * 30 * 24 * 60 * 60, // 2 months in seconds
            release_percentage,
            last_release_timestamp: current_timestamp,
        }
    }

    fn claim_tokens(&mut self) -> ProgramResult {
        let current_timestamp = clock::get().unwrap().unix_timestamp;
        if current_timestamp < self.cliff_timestamp {
            return Err(ProgramError::InvalidAccountData);
        }

        let elapsed_months = (current_timestamp - self.last_release_timestamp) / (30 * 24 * 60 * 60); // Assume 30 days per month
        if elapsed_months == 0 {
            return Err(ProgramError::InvalidAccountData);
        }

        let vested_amount = self.total_tokens * (self.release_percentage as u64) * elapsed_months / 100;
        if vested_amount == 0 {
            return Err(ProgramError::InvalidAccountData);
        }

        self.last_release_timestamp = current_timestamp;
        // Transfer vested tokens to the beneficiary
        // Add your transfer logic here
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

    let admin_account = next_account_info(accounts_iter)?;
    let beneficiary_account = next_account_info(accounts_iter)?;
    let vesting_contract_account = next_account_info(accounts_iter)?;

    let mut vesting_contract = VestingContract::unpack_unchecked(&vesting_contract_account.data.borrow())?;

    // Parse instruction data and dispatch appropriate function
    match instruction_data {
        // Instruction to deposit tokens
        b"deposit_tokens" => {
            let amount_bytes = &instruction_data[0..8];
            let amount = u64::from_le_bytes(amount_bytes.try_into().unwrap());
            vesting_contract.deposit_tokens(admin_account, amount)?;
        }
        // Instruction to claim vested tokens
        b"claim_tokens" => {
            vesting_contract.claim_tokens()?;
        }
        _ => return Err(ProgramError::InvalidInstruction),
    }

    // Update vesting contract account data
    vesting_contract.pack_into(&mut vesting_contract_account.data.borrow_mut())?;

    Ok(())
}

impl VestingContract {
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

    fn deposit_tokens(&mut self, admin_account: &AccountInfo, amount: u64) -> ProgramResult {
        // Add your deposit logic here
        // Example: Transfer tokens from admin's account to the vesting contract's account
        Ok(())
    }
}
