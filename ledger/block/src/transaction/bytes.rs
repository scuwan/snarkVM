// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkVM library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;

impl<N: Network> FromBytes for Transaction<N> {
    /// Reads the transaction from the buffer.
    #[inline]
    fn read_le<R: Read>(reader: R) -> IoResult<Self> {
        // Wrap the reader in a `LimitedReader` with a `MAX_TRANSACTION_SIZE` as a limit.
        let mut reader = LimitedReader::new(reader, N::MAX_TRANSACTION_SIZE);
        // Read the version.
        let version = u8::read_le(&mut reader)?;
        // Ensure the version is valid.
        if version != 1 {
            return Err(error("Invalid transaction version"));
        }

        // Read the variant.
        let variant = u8::read_le(&mut reader)?;
        // Match the variant.
        let (id, transaction) = match variant {
            0 => {
                // Read the ID.
                let id = N::TransactionID::read_le(&mut reader)?;
                // Read the owner.
                let owner = ProgramOwner::read_le(&mut reader)?;
                // Read the deployment.
                let deployment = Deployment::read_le(&mut reader)?;
                // Read the fee.
                let fee = Fee::read_le(&mut reader)?;

                // Initialize the transaction.
                let transaction = Self::from_deployment(owner, deployment, fee).map_err(|e| error(e.to_string()))?;
                // Return the ID and the transaction.
                (id, transaction)
            }
            1 => {
                // Read the ID.
                let id = N::TransactionID::read_le(&mut reader)?;
                // Read the execution.
                let execution = Execution::read_le(&mut reader)?;

                // Read the fee variant.
                let fee_variant = u8::read_le(&mut reader)?;
                // Read the fee.
                let fee = match fee_variant {
                    0u8 => None,
                    1u8 => Some(Fee::read_le(&mut reader)?),
                    _ => return Err(error("Invalid fee variant")),
                };

                // Initialize the transaction.
                let transaction = Self::from_execution(execution, fee).map_err(|e| error(e.to_string()))?;
                // Return the ID and the transaction.
                (id, transaction)
            }
            2 => {
                // Read the ID.
                let id = N::TransactionID::read_le(&mut reader)?;
                // Read the fee.
                let fee = Fee::read_le(&mut reader)?;

                // Initialize the transaction.
                let transaction = Self::from_fee(fee).map_err(|e| error(e.to_string()))?;
                // Return the ID and the transaction.
                (id, transaction)
            }
            3.. => return Err(error("Invalid transaction variant")),
        };

        // Ensure the transaction ID matches.
        match transaction.id() == id {
            // Return the transaction.
            true => Ok(transaction),
            false => Err(error("Transaction ID mismatch")),
        }
    }
}

impl<N: Network> ToBytes for Transaction<N> {
    /// Writes the transaction to the buffer.
    #[inline]
    fn write_le<W: Write>(&self, writer: W) -> IoResult<()> {
        // Wrap the writer in a `LimitedWriter` with a `MAX_TRANSACTION_SIZE` as a limit.
        let mut writer = LimitedWriter::new(writer, N::MAX_TRANSACTION_SIZE);
        // Write the version.
        1u8.write_le(&mut writer)?;

        // Write the transaction.
        match self {
            Self::Deploy(id, owner, deployment, fee) => {
                // Write the variant.
                0u8.write_le(&mut writer)?;
                // Write the ID.
                id.write_le(&mut writer)?;
                // Write the owner.
                owner.write_le(&mut writer)?;
                // Write the deployment.
                deployment.write_le(&mut writer)?;
                // Write the fee.
                fee.write_le(&mut writer)
            }
            Self::Execute(id, execution, fee) => {
                // Write the variant.
                1u8.write_le(&mut writer)?;
                // Write the ID.
                id.write_le(&mut writer)?;
                // Write the execution.
                execution.write_le(&mut writer)?;
                // Write the fee.
                match fee {
                    None => 0u8.write_le(&mut writer),
                    Some(fee) => {
                        1u8.write_le(&mut writer)?;
                        fee.write_le(&mut writer)
                    }
                }
            }
            Self::Fee(id, fee) => {
                // Write the variant.
                2u8.write_le(&mut writer)?;
                // Write the ID.
                id.write_le(&mut writer)?;
                // Write the fee.
                fee.write_le(&mut writer)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type CurrentNetwork = console::network::MainnetV0;

    #[test]
    fn test_bytes() -> Result<()> {
        let rng = &mut TestRng::default();

        for expected in [
            crate::transaction::test_helpers::sample_deployment_transaction(true, rng),
            crate::transaction::test_helpers::sample_deployment_transaction(false, rng),
            crate::transaction::test_helpers::sample_execution_transaction_with_fee(true, rng),
            crate::transaction::test_helpers::sample_execution_transaction_with_fee(false, rng),
        ]
        .into_iter()
        {
            // Check the byte representation.
            let expected_bytes = expected.to_bytes_le()?;
            assert_eq!(expected, Transaction::read_le(&expected_bytes[..])?);
        }
        Ok(())
    }

    #[test]
    fn test_large_transaction_fails() -> Result<()> {
        let rng = &mut TestRng::default();
        // Construct a large execution transaction.
        let transaction = crate::transaction::test_helpers::sample_large_execution_transaction(rng);
        // Check that the execution is larger than the maximum transaction size.
        if let Transaction::Execute(_, execution, _) = &transaction {
            assert!(execution.to_bytes_le().unwrap().len() > CurrentNetwork::MAX_TRANSACTION_SIZE);
        } else {
            unreachable!();
        }
        // Check that `to_bytes_le` fails.
        assert!(transaction.to_bytes_le().is_err());

        // Check that `from_bytes_le` fails.
        let mut bytes_le = Vec::new();
        crate::transaction::test_helpers::unchecked_write_le(&transaction, &mut bytes_le).unwrap();
        // Check that the raw bytes are larger than the maximum transaction size.
        assert!(bytes_le.len() > CurrentNetwork::MAX_TRANSACTION_SIZE);
        assert!(Transaction::<CurrentNetwork>::read_le(&bytes_le[..]).is_err());

        Ok(())
    }
}
