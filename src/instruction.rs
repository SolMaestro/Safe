use solana_program::{
  instruction::{AccountMeta, Instruction},          // For building instructions to send to the blockchain
  pubkey::Pubkey,                                  // For identifying accounts and programs
};
use std::convert::TryInto;                        // Trait from the std lib used to safely convert between types especially when dealing with raw bytes

//Vault Instructions
pub enum VaultInstruction {
  //initialize a new vault
  //Accounts:
  //0. [signer] The vault creator (owner)
  //1. [writable] The vault account (PDA)
  //2. [] The token Mint
  //3. [writable] The vault token account (PDA SPL token account)
  //4. [] Rent sysvar
  //5. [] Token program
  //6. [] System program
  InitVault,

  //Deposit tokens into the vault
  //Accounts:
  //0. [signer] The vault owner
  //1. [writable] Source user token account
  //2. [writable] Vault token account (PDA)
  //3. [] Vault state account
  //4. [] Token program
  Deposit { amount: u64 },

  //Withdraw tokens from vault
  //Accounts:
  //0. [signer] Vault owner
  //1. [writable] Vault token account
  //2. [writable] Destination token account
  //3. [] Vault state account
  //4. [] Token Program
  Withdraw { amount: u64 },
}

impl VaultInstruction {
  //Unpack a byte buffer into a [VaultInstruction].
  pub fn unpack(input: &[u8]) -> Option<Self> {               // Takes a slice of bytes and tries to convert i.e deserialize it into one of the program's instructions
    let (&tag, rest) = input.split_first()?;                  // This line grabs the first byte from the input and puts the rest of the buffer into rest. the first byte usually tells the program which variant to construct.
    Some(match tag {                                          // Pattern matching the tag value to determine which variant of VaultInstruction this should be
      0 => VaultInstruction::InitVault,                       // Initialize vault if it's 0
      1 => {
      // Try to read the next 8 bytes from the input and convert to u64
        let amount = rest
        .get(..8)                                             // Get the first 8 bytes of the rest
        .and_then(|slice| slice.try_into().ok())              // Try to convert &[u8] to [u8; 8]
        .map(u64::from_le_bytes)?;                            // Convert byte array to u64

      VaultInstruction::Deposit {amount}                      // Return the Deposit variant
      }
      2 => {
        let amount = rest
        .get(..8)
        .and_then(|slice| slice.try_into().ok())
        .map(u64::from_le_bytes)?;
      VaultInstruction::Withdraw {amount}
      }
      _ => return None,                                     // If the tag doesn’t match 0, 1, or 2, the input is invalid, returns None
    })
  }
}