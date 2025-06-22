// Core Solana modules for handling programs and accounts
use solana_program::{
  account_info::{next_account_info, AccountInfo},         // Tools to iterate and manage accounts
  entrypoint::ProgramResult,                              // Type for Result<(), ProgramError>
  msg,                                                    // Logging macro for debugging
  program::{invoke, invoke_signed},                       // For making CPI (cross-program invocations)
  program_error::ProgramError,                            // Standard error type
  pubkey::Pubkey,                                         // Public key type used for account IDs
  sysvar::{rent::Rent, Sysvar},                           // Rent system variable for checking rent-exempt status
};

// Import the SPL Token account state definition to interact with token accounts
use spl_token::state::Account as TokenAccount;

// Import your program-specific types
use crate::instruction::VaultInstruction;                 // Custom enum representing supported instructions
use crate::state::Vault;                                  // Vault account struct

// Main entry point for the program's logic
pub fn process_instruction(
  program_id: &Pubkey,                                  // The public key of this program
  accounts: &[AccountInfo],                             // Accounts passed into the transaction
  instruction_data: &[u8],                              // Raw instruction data that will be deserialized into an enum
) -> ProgramResult {
  // Deserialize the instruction data into a VaultInstruction variant
  let instruction = VaultInstruction::unpack(instruction_data).ok_or(ProgramError::InvalidInstructionData)?;

  // Dispatch logic based on which instruction was sent
  match instruction {
    VaultInstruction::InitVault => init_vault(program_id, accounts),                            // Handle vault creation
    VaultInstruction::Deposit { amount } => deposit_tokens(program_id, accounts, amount),       // Handle token deposit
    VaultInstruction::Withdraw { amount } => withdraw_tokens(program_id, accounts, amount),     // Handle token withdrawal
  }
}

fn init_vault(program_id: &Pubkey, accounts: &[AccountInfo],) -> ProgramResult {
  // Create an iterator over the accounts passed into the transaction
  let account_info_iter = &mut accounts.iter();

  // Account 0: The user who initializes the vault must be a signer
  let initializer = next_account_info(account_info_iter)?;

  // Account 1: The vault account (PDA) where vault state will be stored
  let vault_account = next_account_info(account_info_iter)?;

  // Account 2: The SPL token mint this vault is tied to
  let token_mint = next_account_info(account_info_iter)?;

  // Account 3: The associated token account owned by the vault (also likely a PDA)
  let vault_token_account = next_account_info(account_info_iter)?;

   // Account 4: Sysvar account for rent used to check rent exemption
  let rent_sysvar = next_account_info(account_info_iter)?;

   // Account 5: The SPL Token program (for creating/managing token accounts)
  let token_program = next_account_info(account_info_iter)?;

  // Account 6: The system program (for creating system accounts like the vault PDA)
  let system_program = next_account_info(account_info_iter)?;

  // Make sure the initializer actually signed the transaction
  if !initializer.is_signer {
    return Err(ProgramError::MissingRequiredSignature);
  }

  // Try to load (but not validate) the vault account data into a Vault struct
  let mut vault_data = Vault::unpack_unchecked(&vault_account.try_borrow_data()?)?;

   // Make sure we're not reusing an already-initialized vault account
  if vault_data.is_initialized {
    return Err(ProgramError::AccountAlreadyInitialized);
  }

  // Populate the Vault struct with the initial values
  vault_data.is_initialized = true;
  vault_data.owner = *initializer.key;
  vault_data.token_mint = *token_mint.key;
  vault_data.vault_token_account = *vault_token_account.key;

  // Serialize the updated Vault struct back into the vault account's data
  Vault::pack(vault_data, &mut vault_account.try_borrow_mut_data()?)?;

   // Log success message for debugging
  msg!("Vault successfully initialized");

  Ok(())

}