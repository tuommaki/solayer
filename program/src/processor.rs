use {
    crate::{error::SolayerError, instruction::SolayerInstruction},
    borsh::{BorshDeserialize, BorshSerialize},
    bytemuck::{bytes_of, Pod, Zeroable},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        instruction::Instruction,
        msg,
        program::invoke,
        program_error::ProgramError,
        pubkey::Pubkey,
        sysvar::{rent::Rent, Sysvar},
    },
    std::str::FromStr,
};

pub const PUBKEY_SERIALIZED_SIZE: usize = 32;
pub const SIGNATURE_SERIALIZED_SIZE: usize = 64;
pub const SIGNATURE_OFFSETS_SERIALIZED_SIZE: usize = 14;
// bytemuck requires structures to be aligned
pub const SIGNATURE_OFFSETS_START: usize = 2;
pub const DATA_START: usize = SIGNATURE_OFFSETS_SERIALIZED_SIZE + SIGNATURE_OFFSETS_START;

#[derive(Default, Debug, Copy, Clone, Zeroable, Pod)]
#[repr(C)]
pub struct Ed25519SignatureOffsets {
    signature_offset: u16,             // offset to ed25519 signature of 64 bytes
    signature_instruction_index: u16,  // instruction index to find signature
    public_key_offset: u16,            // offset to public key of 32 bytes
    public_key_instruction_index: u16, // instruction index to find public key
    message_data_offset: u16,          // offset to start of message data
    message_data_size: u16,            // size of message data
    message_instruction_index: u16,    // index of instruction data to get message data
}

pub struct Processor;

impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = SolayerInstruction::try_from_slice(instruction_data)?;
        let _account_info_iter = &mut accounts.iter();

        match instruction {
            SolayerInstruction::SigVerify {
                pubkey,
                signature,
                message,
            } => {
                msg!("Instruction: SigVerify");
                assert_eq!(signature.len(), SIGNATURE_SERIALIZED_SIZE);
                Self::process_sig_verify(pubkey, signature, message, program_id)
            }
        }
    }

    fn process_sig_verify(
        pubkey: Pubkey,
        signature: Vec<u8>,
        message: Vec<u8>,
        _program_id: &Pubkey,
    ) -> ProgramResult {
        assert_eq!(signature.len(), SIGNATURE_SERIALIZED_SIZE);
        let i = new_ed25519_instruction(&pubkey, &signature, &message);
        invoke(&i, &[])
    }
}

// From program-sdk
pub fn new_ed25519_instruction(pubkey: &Pubkey, signature: &[u8], message: &[u8]) -> Instruction {
    let mut instruction_data = Vec::with_capacity(
        DATA_START
            .saturating_add(SIGNATURE_SERIALIZED_SIZE)
            .saturating_add(PUBKEY_SERIALIZED_SIZE)
            .saturating_add(message.len()),
    );

    let num_signatures: u8 = 1;
    let public_key_offset = DATA_START;
    let signature_offset = public_key_offset.saturating_add(PUBKEY_SERIALIZED_SIZE);
    let message_data_offset = signature_offset.saturating_add(SIGNATURE_SERIALIZED_SIZE);

    // add padding byte so that offset structure is aligned
    instruction_data.extend_from_slice(bytes_of(&[num_signatures, 0]));

    let offsets = Ed25519SignatureOffsets {
        signature_offset: signature_offset as u16,
        signature_instruction_index: u16::MAX,
        public_key_offset: public_key_offset as u16,
        public_key_instruction_index: u16::MAX,
        message_data_offset: message_data_offset as u16,
        message_data_size: message.len() as u16,
        message_instruction_index: u16::MAX,
    };

    instruction_data.extend_from_slice(bytes_of(&offsets));

    debug_assert_eq!(instruction_data.len(), public_key_offset);

    instruction_data.extend_from_slice(&pubkey.to_bytes());

    debug_assert_eq!(instruction_data.len(), signature_offset);

    instruction_data.extend_from_slice(&signature);

    debug_assert_eq!(instruction_data.len(), message_data_offset);

    instruction_data.extend_from_slice(message);

    Instruction {
        program_id: Pubkey::from_str("Ed25519SigVerify1111111111111111111111111111111111111")
            .unwrap(),
        accounts: vec![],
        data: instruction_data,
    }
}
