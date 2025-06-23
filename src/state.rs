// Import core traits and types from Solana's program SDK
use solana_program::{
  program_pack::{IsInitialized, Pack, Sealed},                            // Traits for (de)serializing account data
  pubkey::Pubkey,                                                         // Solana's public key type for identifying accounts and programs
};

// Import helper macros to safely work with byte arrays often used in manual serialization/deserialization
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};

// Define the Vault struct, this will be the on-chain account structure
pub struct Vault { 
  pub is_initialized: bool,                  // Flag to indicate if the vault account has been initialized
  pub owner: Pubkey,                         // The public key of the vault's owner (authority)
  pub token_mint: Pubkey,                    // The token mint this vault is associated with
  pub vault_token_account: Pubkey,           // The associated token account that will actually hold the tokens
}

// Empty implementation of the Sealed trait, required to implement Pack
impl Sealed for Vault {}                       // This prevents external crates from implementing Pack for Vault


// Implements the IsInitialized trait, which tells Solana if the account is ready for use
impl IsInitialized for Vault {
  fn is_initialized(&self) -> bool {
    self.is_initialized                 // Simply returns the value of the is_initialized field
  }
}

// Implements the Pack trait, which defines how to serialize/deserialize the Vault struct
impl Pack for Vault {
   // Total length of the serialized Vault in bytes
  // 1 byte for bool + 32 for owner + 32 for token_mint + 32 for vault_token_account
  const LEN: usize = 1 + 32 + 32 + 32;

  // Deserialize a Vault struct from a byte slice
  fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {

    // Safely interpret the input slice as an array of Vault::LEN bytes
    let src = array_ref![src, 0, Vault::LEN];

    // Split the slice into its individual fields
    let (is_initialized, owner, token_mint, vault_token_account) = array_ref![src, 1, 32, 32, 32];

    // Construct and return the Vault struct from the split byte fields
    Ok(Vault {
      is_initialized: is_initialized[0] != 0,                                 // Convert byte to bool (non-zero means true)
      owner: Pubkey::new_from_array(*owner),                                  // Convert byte array to Pubkey
      token_mint: Pubkey::new_from_array(*token_mint),
      vault_token_account: Pubkey::new_from_array(*vault_token_account),
    })
  }

  // Implement the method that serializes the Vault struct into a byte slice
  fn pack_into_slice(&self, dst: &mut [u8]) {
  // Safely convert the mutable byte slice into a fixed-size array of Vault::LEN bytes
    let dst = array_mut_ref![dst, 0, Vault::LEN];


    let (
      is_initialized_dst,                 // 1 byte for the bool
      owner_dst,                          // 32 bytes for the owner pubkey
      token_mint_dst,                     // 32 bytes for the mint pubkey
      vault_token_account_dst             // 32 bytes for the mint pubkey
    ) = mut_array_refs![dst, 1, 32, 32, 32];

    
    is_initialized_dst[0] = self.is_initialized as u8;                            // Store is_initialized as 0 or 1

    // Copy the bytes of each Pubkey into their respective destination slices
    owner_dst.copy_from_slice(self.owner.as_ref());
    token_mint_dst.copy_from_slice(self.token_mint.as_ref());
    vault_token_account_dst.copy_from_slice(self.vault_token_account.as_ref());
  }
}

// Structure to hold a user's individual vault state
pub struct UserVault {
  pub is_initialized: bool,                 // Flag to check if the account has been initialized
  pub user: Pubkey,                         // The public key of the depositor i.e the user
  pub vault: Pubkey,                        // The vault this user is interacting with
  pub deposited_amount: u64,                // Total amount this user has deposited
}

// Empty implementation of the Sealed trait, required to implement Pack
impl Sealed for UserVault {}

// Lets Solana programs check if the account is already initialized
impl IsInitialized for UserVault {
fn is_initialized(&self) -> bool {
  self.is_initialized
}
}

// Implement Pack so the struct can be serialized/deserialized into account data
impl Pack for UserVault {
  // The total size of the struct in bytes: 1 (bool) + 32 + 32 + 8 = 73 bytes
  const LEN: usize = 1 + 32 + 32 + 8;

  // Deserialize from raw byte slice into a UserVault struct
  fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
    let src = array_ref![src, 0, UserVault::LEN];

    // Split the byte slice into parts matching the field sizes
    let (is_initialized, user, vault, deposited_amount) = array_refs![src, 1, 32, 32, 8];

    Ok(UserVault{
      is_initialized: is_initialized[0] != 0,                     // Convert byte to bool
      user: Pubkey::new_from_array(*user),                        // Deserialize user pubkey
      vault: Pubkey::new_from_array(*vault),                      // Deserialize vault pubkey
      deposited_amount: u64::from_le_bytes(*deposited_amount),    // Convert 8 bytes to u64
    })
  }

  // Serialize the struct back into a byte slice for writing to the account
  fn pack_into_slice (&self, dst: &mut [u8]) {
    let dst = array_mut_ref![dst, 0, UserVault::LEN];

    // Split the destination slice into pieces for each field
    let (is_initialized_dst, user_dst, vault_dst, deposited_amount_dst) = mut_array_refs![dst, 1, 32, 32, 8];

     // Convert each field into bytes and write it
    is_initialized_dst[0] = self.is_initialized as u8;
    user_dst.copy_from_slice(self.user.as_ref());
    vault_dst.copy_from_slice(self.vault.as_ref());
    *deposited_amount_dst = self.deposited_amount.to_le_bytes();
  }
}
