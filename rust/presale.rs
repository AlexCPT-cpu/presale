use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{clock},
};

entrypoint!(process_instruction);

struct PresaleInfo {
    beneficiary: Pubkey,
    total_tokens: u64,
    token_price: u64,
    sold_tokens: u64,
    presale_closed: bool,
}

impl PresaleInfo {
    fn new(beneficiary: Pubkey, total_tokens: u64, token_price: u64) -> Self {
        Self {
            beneficiary,
            total_tokens,
            token_price,
            sold_tokens: 0,
            presale_closed: false,
        }
    }
}

fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let presale_account = next_account_info(accounts_iter)?;
    let beneficiary_account = next_account_info(accounts_iter)?;
    let token_account = next_account_info(accounts_iter)?;
    let system_program_account = next_account_info(accounts_iter)?;
    let clock_account = next_account_info(accounts_iter)?;

    let mut presale_info = PresaleInfo::unpack_unchecked(&presale_account.data.borrow())?;

    // Check if presale is closed
    if presale_info.presale_closed {
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if presale is sold out
    if presale_info.sold_tokens >= presale_info.total_tokens {
        presale_info.presale_closed = true;
        presale_info.pack_into(&mut presale_account.data.borrow_mut())?;
        return Err(ProgramError::InvalidAccountData);
    }

    // Calculate the number of tokens to purchase
    let remaining_tokens = presale_info.total_tokens - presale_info.sold_tokens;
    let purchase_amount = clock::get()?.unix_timestamp as u64 * presale_info.token_price;
    let tokens_to_purchase = if purchase_amount > remaining_tokens {
        remaining_tokens
    } else {
        purchase_amount
    };

    // Update sold tokens
    presale_info.sold_tokens += tokens_to_purchase;

    // Transfer SOL to beneficiary
    let transfer_sol_ix = system_instruction::transfer(
        token_account.key,
        beneficiary_account.key,
        tokens_to_purchase * presale_info.token_price,
    );
    solana_program::program::invoke(
        &transfer_sol_ix,
        &[token_account.clone(), beneficiary_account.clone(), system_program_account.clone()],
    )?;

    // Transfer tokens to purchaser (for illustration purposes, replace this with actual token transfer)
    let transfer_tokens_ix = system_instruction::transfer(
        token_account.key,
        beneficiary_account.key,
        tokens_to_purchase,
    );
    solana_program::program::invoke(
        &transfer_tokens_ix,
        &[token_account.clone(), beneficiary_account.clone(), system_program_account.clone()],
    )?;

    // Update presale account data
    presale_info.pack_into(&mut presale_account.data.borrow_mut())?;

    Ok(())
}

impl PresaleInfo {
    fn pack_into<'a>(&self, dst: &'a mut [u8]) -> Result<&'a mut [u8], ProgramError> {
        let mut dst = dst;
        let mut src = self.try_to_vec()?;
        let start = dst.len().checked_sub(src.len()).ok_or(ProgramError::InvalidAccountData)?;
        let end = start + src.len();
        dst[start..end].copy_from_slice(&src);
        Ok(dst)
    }

    fn try_to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut buf = Vec::with_capacity(Self::LEN);
        buf.extend_from_slice(&self.beneficiary.to_bytes());
        buf.extend_from_slice(&self.total_tokens.to_le_bytes());
        buf.extend_from_slice(&self.token_price.to_le_bytes());
        buf.extend_from_slice(&self.sold_tokens.to_le_bytes());
        buf.push(self.presale_closed as u8);
        Ok(buf)
    }

    fn unpack_unchecked(src: &[u8]) -> Result<Self, ProgramError> {
        let mut src = src;
        let beneficiary = Pubkey::new_from_array(Self::read_array(&mut src)?);
        let total_tokens = Self::read_u64(&mut src);
        let token_price = Self::read_u64(&mut src);
        let sold_tokens = Self::read_u64(&mut src);
        let presale_closed = src[0] != 0;
        Ok(Self {
            beneficiary,
            total_tokens,
            token_price,
            sold_tokens,
            presale_closed,
        })
    }

    fn read_array(src: &mut &[u8]) -> Result<[u8; 32], ProgramError> {
        let mut arr = [0u8; 32];
        let src_arr = src.get(..32).ok_or(ProgramError::InvalidAccountData)?;
        arr.copy_from_slice(src_arr);
        *src = &src[32..];
        Ok(arr)
    }

    fn read_u64(src: &mut &[u8]) -> u64 {
        let arr = src.get(..8).unwrap_or_default();
        *src = &src[8..];
        u64::from_le_bytes(*arr)
    }
}

impl PresaleInfo {
    const LEN: usize = 1 + 32 + 8 + 8 + 8 + 8;
}
