//! Transaction types for TrinityChain

use sha2::{Digest, Sha256};
use crate::blockchain::{Sha256Hash, TriangleState};
use crate::geometry::Triangle;
use crate::error::ChainError;

pub type Address = String;

/// Maximum transaction size in bytes (100KB) to prevent DoS
pub const MAX_TRANSACTION_SIZE: usize = 100_000;

/// A transaction that can occur in a block
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Transaction {
    Transfer(TransferTx),
    Subdivision(SubdivisionTx),
    Coinbase(CoinbaseTx),
}

impl Transaction {
    pub fn hash_str(&self) -> String {
        hex::encode(self.hash())
    }

    /// Validate transaction size to prevent DoS attacks
    pub fn validate_size(&self) -> Result<(), ChainError> {
        let serialized = bincode::serialize(self)
            .map_err(|e| ChainError::InvalidTransaction(format!("Serialization failed: {}", e)))?;

        if serialized.len() > MAX_TRANSACTION_SIZE {
            return Err(ChainError::InvalidTransaction(
                format!("Transaction too large: {} bytes (max: {})", serialized.len(), MAX_TRANSACTION_SIZE)
            ));
        }
        Ok(())
    }

    /// Get the geometric fee area for this transaction
    pub fn fee_area(&self) -> crate::geometry::Coord {
        match self {
            Transaction::Subdivision(tx) => tx.fee as crate::geometry::Coord,
            Transaction::Transfer(tx) => tx.fee_area,
            Transaction::Coinbase(_) => 0.0, // Coinbase has no fee
        }
    }

    /// Get the fee as u64 (for backward compatibility, converts fee_area)
    /// Deprecated: Use fee_area() for geometric fees
    pub fn fee(&self) -> u64 {
        self.fee_area() as u64
    }

    /// Calculate the hash of this transaction
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        match self {
            Transaction::Subdivision(tx) => {
                hasher.update(tx.parent_hash);
                for child in &tx.children {
                    hasher.update(child.hash());
                }
                hasher.update(tx.owner_address.as_bytes());
                hasher.update(tx.fee.to_le_bytes());
                hasher.update(tx.nonce.to_le_bytes());
            }
            Transaction::Coinbase(tx) => {
                hasher.update("coinbase".as_bytes());
                hasher.update(tx.reward_area.to_le_bytes());
                hasher.update(tx.beneficiary_address.as_bytes());
            }
            Transaction::Transfer(tx) => {
                hasher.update("transfer".as_bytes());
                hasher.update(tx.input_hash);
                hasher.update(tx.new_owner.as_bytes());
                hasher.update(tx.sender.as_bytes());
                hasher.update(tx.fee_area.to_le_bytes());
                hasher.update(tx.nonce.to_le_bytes());
            }
        };
        hasher.finalize().into()
    }

    /// Validate this transaction against the current UTXO state
    pub fn validate(&self, state: &TriangleState) -> Result<(), ChainError> {
        match self {
            Transaction::Subdivision(tx) => tx.validate(state),
            Transaction::Coinbase(tx) => tx.validate(),
            Transaction::Transfer(tx) => tx.validate(),
        }
    }
}

/// Subdivision transaction: splits one parent triangle into three children
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubdivisionTx {
    pub parent_hash: Sha256Hash,
    pub children: Vec<Triangle>,
    pub owner_address: Address,
    pub fee: u64,
    pub nonce: u64,
    pub signature: Option<Vec<u8>>,
    pub public_key: Option<Vec<u8>>,
}

impl SubdivisionTx {
    pub fn new(
        parent_hash: Sha256Hash,
        children: Vec<Triangle>,
        owner_address: Address,
        fee: u64,
        nonce: u64,
    ) -> Self {
        SubdivisionTx {
            parent_hash,
            children,
            owner_address,
            fee,
            nonce,
            signature: None,
            public_key: None,
        }
    }

    pub fn signable_message(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(&self.parent_hash);
        for child in &self.children {
            message.extend_from_slice(&child.hash());
        }
        message.extend_from_slice(self.owner_address.as_bytes());
        message.extend_from_slice(&self.fee.to_le_bytes());
        message.extend_from_slice(&self.nonce.to_le_bytes());
        message
    }

    pub fn sign(&mut self, signature: Vec<u8>, public_key: Vec<u8>) {
        self.signature = Some(signature);
        self.public_key = Some(public_key);
    }

    /// Validates just the signature of the transaction, without access to blockchain state.
    /// This is useful for early validation in the mempool.
    pub fn validate_signature(&self) -> Result<(), ChainError> {
        if self.signature.is_none() || self.public_key.is_none() {
            return Err(ChainError::InvalidTransaction(
                "Transaction not signed".to_string(),
            ));
        }

        let message = self.signable_message();
        let is_valid = crate::crypto::verify_signature(
            self.public_key.as_ref().unwrap(),
            &message,
            self.signature.as_ref().unwrap(),
        )?;

        if !is_valid {
            return Err(ChainError::InvalidTransaction(
                "Invalid signature".to_string(),
            ));
        }

        Ok(())
    }

    /// Performs a full validation of the transaction against the current blockchain state.
    pub fn validate(&self, state: &TriangleState) -> Result<(), ChainError> {
        // First, perform a stateless signature check.
        self.validate_signature()?;

        // Then, validate against the current state (UTXO set).
        if !state.utxo_set.contains_key(&self.parent_hash) {
            return Err(ChainError::TriangleNotFound(format!(
                "Parent triangle {} not found in UTXO set",
                hex::encode(self.parent_hash)
            )));
        }

        let parent = state.utxo_set.get(&self.parent_hash).unwrap();
        let expected_children = parent.subdivide();

        if self.children.len() != 3 {
            return Err(ChainError::InvalidTransaction(
                "Subdivision must produce exactly 3 children".to_string(),
            ));
        }

        for (i, child) in self.children.iter().enumerate() {
            let expected = &expected_children[i];
            if !child.a.equals(&expected.a) ||
               !child.b.equals(&expected.b) ||
               !child.c.equals(&expected.c) {
                return Err(ChainError::InvalidTransaction(format!(
                    "Child {} geometry does not match expected subdivision",
                    i
                )));
            }
        }

        Ok(())
    }
}

/// Coinbase transaction: miner reward
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoinbaseTx {
    pub reward_area: u64,
    pub beneficiary_address: Address,
}

impl CoinbaseTx {
    /// Maximum reward area that can be claimed in a coinbase transaction
    pub const MAX_REWARD_AREA: u64 = 1000;

    pub fn validate(&self) -> Result<(), ChainError> {
        // Validate reward area is within acceptable bounds
        if self.reward_area == 0 {
            return Err(ChainError::InvalidTransaction(
                "Coinbase reward area must be greater than zero".to_string()
            ));
        }

        if self.reward_area > Self::MAX_REWARD_AREA {
            return Err(ChainError::InvalidTransaction(
                format!("Coinbase reward area {} exceeds maximum {}",
                    self.reward_area, Self::MAX_REWARD_AREA)
            ));
        }

        // Validate beneficiary address is not empty
        if self.beneficiary_address.is_empty() {
            return Err(ChainError::InvalidTransaction(
                "Coinbase beneficiary address cannot be empty".to_string()
            ));
        }

        Ok(())
    }
}

/// Transfer transaction - moves ownership of a triangle
/// Fee is now geometric: fee_area is deducted from the triangle's value
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransferTx {
    pub input_hash: Sha256Hash,
    pub new_owner: Address,
    pub sender: Address,
    /// Geometric fee: area deducted from triangle value and given to miner
    pub fee_area: crate::geometry::Coord,
    pub nonce: u64,
    pub signature: Option<Vec<u8>>,
    pub public_key: Option<Vec<u8>>,
    #[serde(default)]
    pub memo: Option<String>,
}

impl TransferTx {
    /// Maximum memo length (256 characters)
    pub const MAX_MEMO_LENGTH: usize = 256;

    /// Geometric tolerance for fee comparisons (matches geometry.rs)
    pub const GEOMETRIC_TOLERANCE: crate::geometry::Coord = 1e-9;

    pub fn new(input_hash: Sha256Hash, new_owner: Address, sender: Address, fee_area: crate::geometry::Coord, nonce: u64) -> Self {
        TransferTx {
            input_hash,
            new_owner,
            sender,
            fee_area,
            nonce,
            signature: None,
            public_key: None,
            memo: None,
        }
    }

    pub fn with_memo(mut self, memo: String) -> Result<Self, ChainError> {
        if memo.len() > Self::MAX_MEMO_LENGTH {
            return Err(ChainError::InvalidTransaction(
                format!("Memo exceeds maximum length of {} characters", Self::MAX_MEMO_LENGTH)
            ));
        }
        self.memo = Some(memo);
        Ok(self)
    }

    pub fn signable_message(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice("TRANSFER:".as_bytes());
        message.extend_from_slice(&self.input_hash);
        message.extend_from_slice(self.new_owner.as_bytes());
        message.extend_from_slice(self.sender.as_bytes());
        // Use f64 bytes for geometric fee
        message.extend_from_slice(&self.fee_area.to_le_bytes());
        message.extend_from_slice(&self.nonce.to_le_bytes());
        message
    }
    
    pub fn sign(&mut self, signature: Vec<u8>, public_key: Vec<u8>) {
        self.signature = Some(signature);
        self.public_key = Some(public_key);
    }
    
    /// Stateless validation: checks signature, addresses, memo, and fee bounds.
    /// Does NOT validate against UTXO state - use validate_with_state() for that.
    pub fn validate(&self) -> Result<(), ChainError> {
        if self.signature.is_none() || self.public_key.is_none() {
            return Err(ChainError::InvalidTransaction("Transfer not signed".to_string()));
        }

        // Validate addresses are not empty
        if self.sender.is_empty() {
            return Err(ChainError::InvalidTransaction("Sender address cannot be empty".to_string()));
        }

        if self.new_owner.is_empty() {
            return Err(ChainError::InvalidTransaction("New owner address cannot be empty".to_string()));
        }

        // Validate fee_area is non-negative and finite
        if !self.fee_area.is_finite() {
            return Err(ChainError::InvalidTransaction("Fee area must be a finite number".to_string()));
        }
        if self.fee_area < 0.0 {
            return Err(ChainError::InvalidTransaction("Fee area cannot be negative".to_string()));
        }

        // Validate memo length to prevent DoS attacks
        if let Some(ref memo) = self.memo {
            if memo.len() > Self::MAX_MEMO_LENGTH {
                return Err(ChainError::InvalidTransaction(
                    format!("Memo exceeds maximum length of {} characters", Self::MAX_MEMO_LENGTH)
                ));
            }
        }

        let message = self.signable_message();
        let is_valid = crate::crypto::verify_signature(
            self.public_key.as_ref().unwrap(),
            &message,
            self.signature.as_ref().unwrap(),
        )?;

        if !is_valid {
            return Err(ChainError::InvalidTransaction("Invalid signature".to_string()));
        }

        Ok(())
    }

    /// Full validation including UTXO state check.
    /// Ensures: input triangle exists AND input.effective_value() > fee_area + TOLERANCE
    pub fn validate_with_state(&self, state: &TriangleState) -> Result<(), ChainError> {
        // First perform stateless validation
        self.validate()?;

        // Check input triangle exists in UTXO set
        let input_triangle = state.utxo_set.get(&self.input_hash).ok_or_else(|| {
            ChainError::TriangleNotFound(format!(
                "Transfer input {} not found in UTXO set",
                hex::encode(self.input_hash)
            ))
        })?;

        // Area balance check: input value must be strictly greater than fee
        // Using tolerance to handle floating-point precision
        let input_value = input_triangle.effective_value();
        let remaining_value = input_value - self.fee_area;

        if remaining_value < Self::GEOMETRIC_TOLERANCE {
            return Err(ChainError::InvalidTransaction(format!(
                "Insufficient triangle value: input has {:.9} but fee_area is {:.9}, leaving {:.9} (minimum: {:.9})",
                input_value, self.fee_area, remaining_value, Self::GEOMETRIC_TOLERANCE
            )));
        }

        // Verify sender owns the triangle
        if input_triangle.owner != self.sender {
            return Err(ChainError::InvalidTransaction(format!(
                "Sender {} does not own input triangle (owned by {})",
                self.sender, input_triangle.owner
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::TriangleState;
    use crate::crypto::KeyPair;
    use crate::geometry::{Point, Triangle};

    #[test]
    fn test_tx_validation_success() {
        let mut state = TriangleState::new();
        let parent = Triangle::new(
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 0.5, y: 0.866 },
            None,
            "test_owner".to_string(),
        );
        let parent_hash = parent.hash();
        state.utxo_set.insert(parent_hash, parent.clone());

        let children = parent.subdivide();
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();

        let mut tx = SubdivisionTx::new(parent_hash, children.to_vec(), address, 0, 1);
        let message = tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature, public_key);

        assert!(tx.validate(&state).is_ok());
    }

    #[test]
    fn test_unsigned_transaction_fails() {
        let mut state = TriangleState::new();
        let parent = Triangle::new(
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 0.5, y: 0.866 },
            None,
            "test_owner".to_string(),
        );
        let parent_hash = parent.hash();
        state.utxo_set.insert(parent_hash, parent.clone());

        let children = parent.subdivide();
        let address = "test_address".to_string();

        let tx = SubdivisionTx::new(parent_hash, children.to_vec(), address, 0, 1);
        assert!(tx.validate(&state).is_err());
    }

    #[test]
    fn test_invalid_signature_fails() {
        let mut state = TriangleState::new();
        let parent = Triangle::new(
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 0.5, y: 0.866 },
            None,
            "test_owner".to_string(),
        );
        let parent_hash = parent.hash();
        state.utxo_set.insert(parent_hash, parent.clone());

        let children = parent.subdivide();
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();

        let mut tx = SubdivisionTx::new(parent_hash, children.to_vec(), address, 0, 1);
        let fake_signature = vec![0u8; 64];
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(fake_signature, public_key);

        assert!(tx.validate(&state).is_err());
    }

    #[test]
    fn test_tx_validation_area_conservation_failure() {
        let mut state = TriangleState::new();
        let parent = Triangle::new(
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 0.5, y: 0.866 },
            None,
            "test_owner".to_string(),
        );
        let parent_hash = parent.hash();
        state.utxo_set.insert(parent_hash, parent);

        let bad_child = Triangle::new(
            Point { x: 0.0, y: 0.0 },
            Point { x: 2.0, y: 0.0 },
            Point { x: 1.0, y: 1.732 },
            None,
            "test_owner".to_string(),
        );
        let children = vec![bad_child.clone(), bad_child.clone(), bad_child];

        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();

        let tx = SubdivisionTx::new(parent_hash, children, address, 0, 1);
        assert!(tx.validate(&state).is_err());
    }

    #[test]
    fn test_tx_validation_double_spend_check() {
        let state = TriangleState::new();

        let parent = Triangle::new(
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 0.5, y: 0.866 },
            None,
            "test_owner".to_string(),
        );
        let parent_hash = parent.hash();
        let children = parent.subdivide();

        let address = "test_address".to_string();
        let tx = SubdivisionTx::new(parent_hash, children.to_vec(), address, 0, 1);

        assert!(tx.validate(&state).is_err());
    }

    #[test]
    fn test_geometric_fee_deduction() {
        // Test case: Start with a large triangle (area ~10.0), transfer with fee_area 0.0001
        // After transfer, the resulting triangle must have value = 10.0 - 0.0001

        // Create a right triangle with area = 10.0 (base=4, height=5 -> area = 0.5*4*5 = 10)
        let mut state = TriangleState::new();
        let keypair = KeyPair::generate().unwrap();
        let sender_address = keypair.address();

        // Triangle with area exactly 10.0: vertices at (0,0), (4,0), (0,5)
        // Area = 0.5 * base * height = 0.5 * 4 * 5 = 10.0
        let large_triangle = Triangle::new(
            Point { x: 0.0, y: 0.0 },
            Point { x: 4.0, y: 0.0 },
            Point { x: 0.0, y: 5.0 },
            None,
            sender_address.clone(),
        );

        let triangle_hash = large_triangle.hash();
        let triangle_area = large_triangle.area();
        assert!((triangle_area - 10.0).abs() < 1e-9, "Test setup: triangle should have area 10.0");

        state.utxo_set.insert(triangle_hash, large_triangle);

        // Create a transfer transaction with geometric fee
        let fee_area: f64 = 0.0001;
        let recipient_address = "recipient_address".to_string();

        let mut tx = TransferTx::new(
            triangle_hash,
            recipient_address.clone(),
            sender_address.clone(),
            fee_area,
            1,
        );

        // Sign the transaction
        let message = tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature, public_key);

        // Validate the transaction against state
        assert!(tx.validate_with_state(&state).is_ok(), "Transfer should be valid");

        // Simulate what apply_block does:
        // 1. Remove old triangle
        let old_triangle = state.utxo_set.remove(&triangle_hash).unwrap();
        let old_value = old_triangle.effective_value();
        let new_value = old_value - fee_area;

        // 2. Create new triangle with reduced value
        let new_triangle = Triangle::new_with_value(
            old_triangle.a,
            old_triangle.b,
            old_triangle.c,
            old_triangle.parent_hash,
            recipient_address.clone(),
            new_value,
        );

        let new_hash = new_triangle.hash();
        state.utxo_set.insert(new_hash, new_triangle);

        // 3. Verify the result
        let result_triangle = state.utxo_set.get(&new_hash).unwrap();

        // Assert: new owner is the recipient
        assert_eq!(result_triangle.owner, recipient_address);

        // Assert: effective value is exactly 10.0 - 0.0001 = 9.9999
        let expected_value = 10.0 - 0.0001;
        let actual_value = result_triangle.effective_value();
        assert!(
            (actual_value - expected_value).abs() < 1e-12,
            "After fee deduction, triangle value should be {:.9}, got {:.9}",
            expected_value,
            actual_value
        );

        // Assert: geometric area is unchanged (still 10.0)
        let geometric_area = result_triangle.area();
        assert!(
            (geometric_area - 10.0).abs() < 1e-9,
            "Geometric area should remain 10.0, got {:.9}",
            geometric_area
        );
    }

    #[test]
    fn test_geometric_fee_insufficient_value() {
        // Test that a fee larger than the triangle value fails validation
        let mut state = TriangleState::new();
        let keypair = KeyPair::generate().unwrap();
        let sender_address = keypair.address();

        // Small triangle with area ~0.5
        let small_triangle = Triangle::new(
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 0.5, y: 1.0 },
            None,
            sender_address.clone(),
        );

        let triangle_hash = small_triangle.hash();
        let triangle_area = small_triangle.area();
        state.utxo_set.insert(triangle_hash, small_triangle);

        // Try to pay a fee larger than the triangle area
        let fee_area = triangle_area + 0.1; // More than available

        let mut tx = TransferTx::new(
            triangle_hash,
            "recipient".to_string(),
            sender_address.clone(),
            fee_area,
            1,
        );

        let message = tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature, public_key);

        // This should fail validation due to insufficient value
        let result = tx.validate_with_state(&state);
        assert!(result.is_err(), "Transfer with fee > value should fail");

        if let Err(ChainError::InvalidTransaction(msg)) = result {
            assert!(msg.contains("Insufficient"), "Error should mention insufficient value");
        } else {
            panic!("Expected InvalidTransaction error");
        }
    }

    #[test]
    fn test_negative_fee_rejected() {
        // Test that negative fees are rejected in stateless validation
        let keypair = KeyPair::generate().unwrap();

        let mut tx = TransferTx::new(
            [0u8; 32],
            "recipient".to_string(),
            keypair.address(),
            -1.0, // Negative fee
            1,
        );

        let message = tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature, public_key);

        let result = tx.validate();
        assert!(result.is_err(), "Negative fee should be rejected");
    }
}
