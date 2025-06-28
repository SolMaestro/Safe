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

fn deposit_tokens(
  program_id: &Pubkey,                                 // Public key of the program
  accounts: &[accounts],                                // The list of accounts passed to the instruction
  amount: u64,                                          // The amount or number of tokens to deposit
) -> ProgramResult {
  // Create a mutable iterator over the accounts list so that each account can be processed in order
  let account_info_iter = &mut accounts.iter();          

  let depositor = next_account_info(account_info_iter)?;                    // The user initiating the deposit
  let user_source_token_account = next_account_info(account_info_iter)?;    // The user's token account holding the tokens to be deposited
  let vault_token_account = next_account_info(account_info_iter)?;          // The vault's token account where the tokens will be sent
  let vault_state_account = next_account_info(account_info_iter)?;          // The account holding the vault's state/configuration data
  let user_vault_account = next_account_info(account_info_iter)?;           // New PDA account
  let token_program = next_account_info(account_info_iter)?;                // The SPL Token program required for token transfer

  // Check that the depositor signed the transaction to prevent unauthorized access
  if !depositor.is_signer {
    return Err(ProgramError::MissingRequiredSignature);
  }

  // Deserialize the vault state account into a Vault struct
  let mut vault = Vault::unpack(&vault_state_account.try_borrow_data()?)?;

  // Safely increment the vault's total_deposits by the new deposit amount. `checked_add` protects against overflow; returns error if overflow would occur.
  vault.total_deposits = vault.total_deposits.checked_add(amount).ok_or(ProgramError::InvalidInstructionData)?;

  // Save (pack) the updated vault state back into the vault_state_account's data. `try_borrow_mut_data` ensures we're safely getting a mutable reference to the account's data.
  Vault::pack(vault, &mut vault_state_account.try_borrow_mut_data()?)?;

  // Derive the expected PDA for the user's vault account. Seeds for include "user_vault", depositor pubkey, and vault state pubkey.
  // This ensures a unique address per user-vault combination and program.
  let (expected_user_vault_pda, _bump) = Pubkey::find_program_address(
    &[b"user_vault", depositor.key.as_ref(), vault_state_account.key.as_ref()],
    program_id,
  );

  // Check if the derived PDA matches the actual provided user_vault_account. This ensures the user isn't trying to spoof a different PDA.
  if expected_user_vault_pda != *user_vault_account.key {
    return Err(ProgramError::InvalidAccountData);
  }

  // Handle initialization or loading of the user's vault data. If the user vault account is empty (first-time depositor), initialize it.
  let mut user_vault_data = if user_vault_account.data_is_empty() {
    UserVault {
      is_initialized: true,
      user: *depositor.key,
      vault: *vault_state_account.key,
      deposited_amount: 0,
    }
  } else {
    // Otherwise, unpack the existing user vault data from the account.
    UserVault::unpack(&user_vault_account.try_borrow_data()?)?
  };
  
  // Build the SPL Token transfer instruction
  // This will transfer `amount` tokens from the user's token account to the vault token account
  let transfer_ix = spl_token::instruction::transfer(
    token_program.key,                             // SPL Token program ID
    user_source_token_account.key,                 // Source token account of user
    vault_token_account.key,                       // Destination token account (vault's)
    depositor.key,                                 // Authority account that must sign
    &[],                                           // For implementing multi-signers (empty for now)
    amount,                                        // Amount of tokens to deposit to vault
  )?;

  // Actually invoke the transfer instruction inside this program. This is a Cross-Program Invocation (CPI) to the Token program
  invoke(
    &transfer_ix,
    &[
      user_source_token_account.clone(),              // Source account
      vault_token_account.clone(),                    // Destination account
      depositor.clone(),                              // Authority account
      token_program.clone(),                          // SPL Token program
    ]
  )?;

  // Safely add the deposit amount to the user's personal deposited amount. As usual `checked_add` again avoids overflow and ensures safe arithmetic.
  user_vault_data.deposited_amount = user_vault_data
  .deposited_amount
  .checked_add(amount)
  .ok_or(ProgramError::InvalidInstructionData)?;

  // Write (serialize) the updated user vault struct back into the user_vault_account data. This persists the updated user deposit to Solana storage.
  UserVault::pack(user_vault_data, &mut user_vault_account.try_borrow_mut_data()?)?;

  // Log a message indicating the deposit was successful plus the actual amount deposited
  msg!("{} tokens deposited by {}", amount, depositor.key);

  Ok(())
}

fn withdraw_tokens(program_id: &Pubkey, accounts: &[accounts], amount: u64) -> ProgramResult {
  let account_info_iter = &mut accounts.iter();

  let user = next_account_info(account_info_iter)?;
  let vault_token_account = next_account_info(account_info_iter)?;
  let user_destination_token_account = next_account_info(account_info_iter)?;
  let vault_state = next_account_info(account_info_iter)?;
  let user_vault_account = next_account_info(account_info_iter)?;
  let token_program = next_account_info(account_info_iter)?;

  if !user.is_signer {
    return Err(ProgramError::MissingRequiredSignature);
  }

  // Load the current vault state from its account data
  let mut vault = Vault::unpack(&vault_state_account.try_borrow_data()?)?;

  // Safely subtract the withdrawal amount from the vault's total deposits. If the vault doesn’t have enough funds recorded, return an error
  vault.total_deposits = vault.total_deposits.checked_sub(amount).ok_or(ProgramError::InsufficientFunds)?;

  // Save the updated vault state back into the account data
  Vault::pack(vault, &mut vault_state_account.try_borrow_mut_data()?)?;

  // Recompute the expected PDA for the user's vault account using seeds. This ensures the client isn't passing in a spoofed or incorrect account
  let (expected_pda, _bump) = Pubkey::find_program_address(
    &[b"user_vault", user.key.as_ref(), vault_state_account.key.as_ref()],
    program_id,
  );

  // Validate that the expected PDA matches the provided user vault account
  if expected_pda != *user_vault_account.key {
    return Err(ProgramError::InvalidAccountData);
  }

  // Load the user's vault record.
  let mut user_vault = UserVault::unpack(&user_vault_account.try_borrow_data()?)?;

  // Ensure the user has enough tokens deposited to withdraw the requested amount
  if user_vault.deposited_amount < amount {
    return Err(ProgramError::InsufficientFunds);
  }

  // Subtract the withdrawal amount from the user's deposited balance
  user_vault.deposited_amount -= amount;

  // Save the updated user state back into the user vault account
  UserVault::pack(user_vault, &mut user_vault_account.try_borrow_mut_data()?)?;

  // Derive the vault authority PDA, which will sign the token transfer.
  let (vault_authority, bump_seed) = Pubkey::find_program_address(&[b"vault"], program_id);

  // Prepare the signer seeds used for invoke_signed, it must match the PDA derivation
  let seeds = &[b"vault", &[bump_seed]];

  // Construct a token program transfer instruction to send tokens from vault to user.
  let transfer_ix = spl_token::instruction::transfer(
    token_program.key,
    vault_token_account.key,                          // Vault_token_account = source which is the vault's token holding account
    user_destination_token_account.key,               // User_destination_token_account which is user's receiving account
    &vault_authority,                                 // Vault_authority = the signer (PDA that owns the vault_token_account). Authority is a PDA, so needs invoke_signed
    &[],                                              // No additional signers needed for now
    amount,
  )?;

  // Execute the token transfer with PDA signing via invoke_signed.
  invoke_signed(
    &transfer_ix,
    &[
      vault_token_account.clone(),
      user_destination_token_account.clone(),
      token_program.clone(),
    ],
   &[seeds],                                    // Signer seeds used to authorize PDA
  )?;

  // Log a message for off-chain indexing or debugging.
  msg!("{} tokens withdrawn by {}", amount, user.key);

  Ok(())
}