//! Instruction types

use crate::error::TokenError;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    program_option::COption,
    pubkey::Pubkey,
    sysvar,
};
use std::convert::TryInto;
use std::mem::size_of;

pub const MIN_SIGNERS: usize = 1;
pub const MAX_SIGNERS: usize = 11;

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum TokenInstruction {
    InitializeMint {
        decimals: u8,
        mint_authority: Pubkey,
        freeze_authority: COption<Pubkey>,
    },
    InitializeAccount,
    // InitializeMultisig {
    //    m: u8,
    // },
    Transfer {
        amount: u64,
    },
    Approve {
        amount: u64,
    },
    Revoke,
    SetAuthority {
        authority_type: AuthorityType,
        new_authority: COption<Pubkey>,
    },
    MintTo {
        amount: u64,
    },
    Burn {
        amount: u64,
    },
    CloseAccount,
    FreezeAccount,
    ThawAccount,

    TransferChecked {
        amount: u64,
        decimals: u8,
    },
    ApproveChecked {
        amount: u64,
        decimals: u8,
    },
    MintToChecked {
        amount: u64,
        decimals: u8,
    },
    BurnChecked {
        amount: u64,
        decimals: u8,
    },
    InitializeAccount2 {
        owner: Pubkey,
    },
}
impl TokenInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        use TokenError::InvalidInstruction;

        let (&tag, rest) = input.split_first().ok_or(InvalidInstruction)?;
        Ok(match tag {
            0 => {
                let (&decimals, rest) = rest.split_first().ok_or(InvalidInstruction)?;
                let (mint_authority, rest) = Self::unpack_pubkey(rest)?;
                let (freeze_authority, _rest) = Self::unpack_pubkey_option(rest)?;
                Self::InitializeMint {
                    mint_authority,
                    freeze_authority,
                    decimals,
                }
            }
            1 => Self::InitializeAccount,
            3 | 4 | 7 | 8 => {
                let amount = rest
                    .get(..8)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;
                match tag {
                    3 => Self::Transfer { amount },
                    4 => Self::Approve { amount },
                    7 => Self::MintTo { amount },
                    8 => Self::Burn { amount },
                    _ => unreachable!(),
                }
            }
            5 => Self::Revoke,
            6 => {
                let (authority_type, rest) = rest
                    .split_first()
                    .ok_or_else(|| ProgramError::from(InvalidInstruction))
                    .and_then(|(&t, rest)| Ok((AuthorityType::from(t)?, rest)))?;
                let (new_authority, _rest) = Self::unpack_pubkey_option(rest)?;

                Self::SetAuthority {
                    authority_type,
                    new_authority,
                }
            }
            9 => Self::CloseAccount,
            10 => Self::FreezeAccount,
            11 => Self::ThawAccount,
            12 => {
                let (amount, rest) = rest.split_at(8);
                let amount = amount
                    .try_into()
                    .ok()
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;
                let (&decimals, _rest) = rest.split_first().ok_or(InvalidInstruction)?;

                Self::TransferChecked { amount, decimals }
            }
            13 => {
                let (amount, rest) = rest.split_at(8);
                let amount = amount
                    .try_into()
                    .ok()
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;
                let (&decimals, _rest) = rest.split_first().ok_or(InvalidInstruction)?;

                Self::ApproveChecked { amount, decimals }
            }
            14 => {
                let (amount, rest) = rest.split_at(8);
                let amount = amount
                    .try_into()
                    .ok()
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;
                let (&decimals, _rest) = rest.split_first().ok_or(InvalidInstruction)?;

                Self::MintToChecked { amount, decimals }
            }
            15 => {
                let (amount, rest) = rest.split_at(8);
                let amount = amount
                    .try_into()
                    .ok()
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;
                let (&decimals, _rest) = rest.split_first().ok_or(InvalidInstruction)?;

                Self::BurnChecked { amount, decimals }
            }
            16 => {
                let (owner, _rest) = Self::unpack_pubkey(rest)?;
                Self::InitializeAccount2 { owner }
            }

            _ => return Err(TokenError::InvalidInstruction.into()),
        })
    }

    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match self {
            &Self::InitializeMint {
                ref mint_authority,
                ref freeze_authority,
                decimals,
            } => {
                buf.push(0);
                buf.push(decimals);
                buf.extend_from_slice(mint_authority.as_ref());
                Self::pack_pubkey_option(freeze_authority, &mut buf);
            }
            Self::InitializeAccount => buf.push(1),
            // &Self::InitializeMultisig { m } => {
            //    buf.push(2);
            //    buf.push(m);
            // }
            &Self::Transfer { amount } => {
                buf.push(3);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            &Self::Approve { amount } => {
                buf.push(4);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            &Self::MintTo { amount } => {
                buf.push(7);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            &Self::Burn { amount } => {
                buf.push(8);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            Self::Revoke => buf.push(5),
            Self::SetAuthority {
                authority_type,
                ref new_authority,
            } => {
                buf.push(6);
                buf.push(authority_type.into());
                Self::pack_pubkey_option(new_authority, &mut buf);
            }
            Self::CloseAccount => buf.push(9),
            Self::FreezeAccount => buf.push(10),
            Self::ThawAccount => buf.push(11),
            &Self::TransferChecked { amount, decimals } => {
                buf.push(12);
                buf.extend_from_slice(&amount.to_le_bytes());
                buf.push(decimals);
            }
            &Self::ApproveChecked { amount, decimals } => {
                buf.push(13);
                buf.extend_from_slice(&amount.to_le_bytes());
                buf.push(decimals);
            }
            &Self::MintToChecked { amount, decimals } => {
                buf.push(14);
                buf.extend_from_slice(&amount.to_le_bytes());
                buf.push(decimals);
            }
            &Self::BurnChecked { amount, decimals } => {
                buf.push(15);
                buf.extend_from_slice(&amount.to_le_bytes());
                buf.push(decimals);
            }
            &Self::InitializeAccount2 { owner } => {
                buf.push(16);
                buf.extend_from_slice(owner.as_ref());
            }
        };
        buf
    }

    fn unpack_pubkey(input: &[u8]) -> Result<(Pubkey, &[u8]), ProgramError> {
        if input.len() >= 32 {
            let (key, rest) = input.split_at(32);
            let pk = Pubkey::new(key);
            Ok((pk, rest))
        } else {
            Err(TokenError::InvalidInstruction.into())
        }
    }

    fn unpack_pubkey_option(input: &[u8]) -> Result<(COption<Pubkey>, &[u8]), ProgramError> {
        match input.split_first() {
            Option::Some((&0, rest)) => Ok((COption::None, rest)),
            Option::Some((&1, rest)) if rest.len() >= 32 => {
                let (key, rest) = rest.split_at(32);
                let pk = Pubkey::new(key);
                Ok((COption::Some(pk), rest))
            }
            _ => Err(TokenError::InvalidInstruction.into()),
        }
    }

    fn pack_pubkey_option(value: &COption<Pubkey>, buf: &mut Vec<u8>) {
        match *value {
            COption::Some(ref key) => {
                buf.push(1);
                buf.extend_from_slice(&key.to_bytes());
            }
            COption::None => buf.push(0),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq)]
pub enum AuthorityType {
    MintTokens,
    FreezeAccount,
    AccountOwner,
    CloseAccount,
}

impl AuthorityType {
    fn into(&self) -> u8 {
        match self {
            AuthorityType::MintTokens => 0,
            AuthorityType::FreezeAccount => 1,
            AuthorityType::AccountOwner => 2,
            AuthorityType::CloseAccount => 3,
        }
    }

    fn from(index: u8) -> Result<Self, ProgramError> {
        match index {
            0 => Ok(AuthorityType::MintTokens),
            1 => Ok(AuthorityType::FreezeAccount),
            2 => Ok(AuthorityType::AccountOwner),
            3 => Ok(AuthorityType::CloseAccount),
            _ => Err(TokenError::InvalidInstruction.into()),
        }
    }
}

pub fn initialize_mint(
    token_program_id: &Pubkey,
    mint_pubkey: &Pubkey,
    mint_authority_pubkey: &Pubkey,
    freeze_authority_pubkey: Option<&Pubkey>,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    let freeze_authority = freeze_authority_pubkey.cloned().into();
    let data = TokenInstruction::InitializeMint {
        mint_authority: *mint_authority_pubkey,
        freeze_authority,
        decimals,
    }
    .pack();

    let accounts = vec![
        AccountMeta::new(*mint_pubkey, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

pub fn initialize_account(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::InitializeAccount.pack();

    let accounts = vec![
        AccountMeta::new(*account_pubkey, false),
        AccountMeta::new_readonly(*mint_pubkey, false),
        AccountMeta::new_readonly(*owner_pubkey, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

pub fn initialize_account2(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::InitializeAccount2 {
        owner: *owner_pubkey,
    }
    .pack();

    let accounts = vec![
        AccountMeta::new(*account_pubkey, false),
        AccountMeta::new_readonly(*mint_pubkey, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

//pub fn initialize_multisig(
//    token_program_id: &Pubkey,
//    multisig_pubkey: &Pubkey,
//    signer_pubkeys: &[&Pubkey],
//    m: u8,
//) -> Result<Instruction, ProgramError> {
//    if !is_valid_signer_index(m as usize)
//        || !is_valid_signer_index(signer_pubkeys.len())
//        || m as usize > signer_pubkeys.len()
//    {
//        return Err(ProgramError::MissingRequiredSignature);
//    }
//    let data = TokenInstruction::InitializeMultisig { m }.pack();
//
//    let mut accounts = Vec::with_capacity(1 + 1 + signer_pubkeys.len());
//    accounts.push(AccountMeta::new(*multisig_pubkey, false));
//    accounts.push(AccountMeta::new_readonly(sysvar::rent::id(), false));
//    for signer_pubkey in signer_pubkeys.iter() {
//        accounts.push(AccountMeta::new_readonly(**signer_pubkey, false));
//    }
//
//    Ok(Instruction {
//        program_id: *token_program_id,
//        accounts,
//        data,
//    })
//}

pub fn transfer(
    token_program_id: &Pubkey,
    source_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::Transfer { amount }.pack();

    let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*source_pubkey, false));
    accounts.push(AccountMeta::new(*destination_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *authority_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

pub fn approve(
    token_program_id: &Pubkey,
    source_pubkey: &Pubkey,
    delegate_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::Approve { amount }.pack();

    let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*source_pubkey, false));
    accounts.push(AccountMeta::new_readonly(*delegate_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *owner_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

pub fn revoke(
    token_program_id: &Pubkey,
    source_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::Revoke.pack();

    let mut accounts = Vec::with_capacity(2 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*source_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *owner_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

pub fn set_authority(
    token_program_id: &Pubkey,
    owned_pubkey: &Pubkey,
    new_authority_pubkey: Option<&Pubkey>,
    authority_type: AuthorityType,
    owner_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    let new_authority = new_authority_pubkey.cloned().into();
    let data = TokenInstruction::SetAuthority {
        authority_type,
        new_authority,
    }
    .pack();

    let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*owned_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *owner_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

pub fn mint_to(
    token_program_id: &Pubkey,
    mint_pubkey: &Pubkey,
    account_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::MintTo { amount }.pack();

    let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*mint_pubkey, false));
    accounts.push(AccountMeta::new(*account_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *owner_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

pub fn burn(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::Burn { amount }.pack();

    let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*account_pubkey, false));
    accounts.push(AccountMeta::new(*mint_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *authority_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

pub fn close_account(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::CloseAccount.pack();

    let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*account_pubkey, false));
    accounts.push(AccountMeta::new(*destination_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *owner_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

pub fn freeze_account(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::FreezeAccount.pack();

    let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*account_pubkey, false));
    accounts.push(AccountMeta::new_readonly(*mint_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *owner_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

pub fn thaw_account(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::ThawAccount.pack();

    let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*account_pubkey, false));
    accounts.push(AccountMeta::new_readonly(*mint_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *owner_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn transfer_checked(
    token_program_id: &Pubkey,
    source_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::TransferChecked { amount, decimals }.pack();

    let mut accounts = Vec::with_capacity(4 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*source_pubkey, false));
    accounts.push(AccountMeta::new_readonly(*mint_pubkey, false));
    accounts.push(AccountMeta::new(*destination_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *authority_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn approve_checked(
    token_program_id: &Pubkey,
    source_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    delegate_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::ApproveChecked { amount, decimals }.pack();

    let mut accounts = Vec::with_capacity(4 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*source_pubkey, false));
    accounts.push(AccountMeta::new_readonly(*mint_pubkey, false));
    accounts.push(AccountMeta::new_readonly(*delegate_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *owner_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

pub fn mint_to_checked(
    token_program_id: &Pubkey,
    mint_pubkey: &Pubkey,
    account_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::MintToChecked { amount, decimals }.pack();

    let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*mint_pubkey, false));
    accounts.push(AccountMeta::new(*account_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *owner_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

pub fn burn_checked(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    let data = TokenInstruction::BurnChecked { amount, decimals }.pack();

    let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
    accounts.push(AccountMeta::new(*account_pubkey, false));
    accounts.push(AccountMeta::new(*mint_pubkey, false));
    accounts.push(AccountMeta::new_readonly(
        *authority_pubkey,
        signer_pubkeys.is_empty(),
    ));
    for signer_pubkey in signer_pubkeys.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Ok(Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    })
}

pub fn is_valid_signer_index(index: usize) -> bool {
    (MIN_SIGNERS..=MAX_SIGNERS).contains(&index)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_instruction_packing() {
        let check = TokenInstruction::InitializeMint {
            decimals: 2,
            mint_authority: Pubkey::new(&[1u8; 32]),
            freeze_authority: COption::None,
        };
        let packed = check.pack();
        let mut expect = Vec::from([0u8, 2]);
        expect.extend_from_slice(&[1u8; 32]);
        expect.extend_from_slice(&[0]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::InitializeMint {
            decimals: 2,
            mint_authority: Pubkey::new(&[2u8; 32]),
            freeze_authority: COption::Some(Pubkey::new(&[3u8; 32])),
        };
        let packed = check.pack();
        let mut expect = vec![0u8, 2];
        expect.extend_from_slice(&[2u8; 32]);
        expect.extend_from_slice(&[1]);
        expect.extend_from_slice(&[3u8; 32]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::InitializeAccount;
        let packed = check.pack();
        let expect = Vec::from([1u8]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        // let check = TokenInstruction::InitializeMultisig { m: 1 };
        // let packed = check.pack();
        // let expect = Vec::from([2u8, 1]);
        // assert_eq!(packed, expect);
        // let unpacked = TokenInstruction::unpack(&expect).unwrap();
        // assert_eq!(unpacked, check);

        let check = TokenInstruction::Transfer { amount: 1 };
        let packed = check.pack();
        let expect = Vec::from([3u8, 1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::Approve { amount: 1 };
        let packed = check.pack();
        let expect = Vec::from([4u8, 1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::Revoke;
        let packed = check.pack();
        let expect = Vec::from([5u8]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::SetAuthority {
            authority_type: AuthorityType::FreezeAccount,
            new_authority: COption::Some(Pubkey::new(&[4u8; 32])),
        };
        let packed = check.pack();
        let mut expect = Vec::from([6u8, 1]);
        expect.extend_from_slice(&[1]);
        expect.extend_from_slice(&[4u8; 32]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::MintTo { amount: 1 };
        let packed = check.pack();
        let expect = Vec::from([7u8, 1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::Burn { amount: 1 };
        let packed = check.pack();
        let expect = Vec::from([8u8, 1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::CloseAccount;
        let packed = check.pack();
        let expect = Vec::from([9u8]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::FreezeAccount;
        let packed = check.pack();
        let expect = Vec::from([10u8]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::ThawAccount;
        let packed = check.pack();
        let expect = Vec::from([11u8]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::TransferChecked {
            amount: 1,
            decimals: 2,
        };
        let packed = check.pack();
        let expect = Vec::from([12u8, 1, 0, 0, 0, 0, 0, 0, 0, 2]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::ApproveChecked {
            amount: 1,
            decimals: 2,
        };
        let packed = check.pack();
        let expect = Vec::from([13u8, 1, 0, 0, 0, 0, 0, 0, 0, 2]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::MintToChecked {
            amount: 1,
            decimals: 2,
        };
        let packed = check.pack();
        let expect = Vec::from([14u8, 1, 0, 0, 0, 0, 0, 0, 0, 2]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::BurnChecked {
            amount: 1,
            decimals: 2,
        };
        let packed = check.pack();
        let expect = Vec::from([15u8, 1, 0, 0, 0, 0, 0, 0, 0, 2]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = TokenInstruction::InitializeAccount2 {
            owner: Pubkey::new(&[2u8; 32]),
        };
        let packed = check.pack();
        let mut expect = vec![16u8];
        expect.extend_from_slice(&[2u8; 32]);
        assert_eq!(packed, expect);
        let unpacked = TokenInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);
    }
}
