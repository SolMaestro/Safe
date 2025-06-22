// Import essential types and modules from the Solana runtime 

use solana_program::{
  account_info::AccountInfo,              // Represents an account's metadata (key, owner, data, etc.)
  entrypoint,                             // Macro to define the program's entry point
  entrypoint::ProgramResult,              // Standard return type for Solana program functions
  pubkey::Pubkey,                         // Public key type used across Solana ( for accounts, owners)
};

// Declare separate modules for organization and maintainability

pub mod instruction;                            // Defines custom instruction data formats (e.g., VaultCreate, VaultDeposit)
pub mod processor;                             // Contains the core logic for handling instructions
pub mod state;                                // Defines the accounts (data structures) used in the program, e.g., Vault

use processor::process_instruction;           // Bring the process_instruction function into scope from the processor module

entrypoint!(process_instruction_entry);      // Define the program's entry point using the Solana macro

// The actual entry function that gets called when a transaction is sent to the program

fn process_instruction_entry(
  program_id: &Pubkey,                                                  // The program ID that owns this execution context
  accounts: &[AccountInfo],                                             // Array of accounts involved in the transaction
  instruction_data: &[u8],                                             // Raw instruction data (usually deserialized into the custom instruction enum)
) -> ProgramResult {

  // Delegate the real processing work to the custom handler
    process_instruction(program_id, accounts, instruction_data)
}