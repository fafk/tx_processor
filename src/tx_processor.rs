use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serializer};
use serde::Serialize;
use bigdecimal::{BigDecimal, Zero};
use std::error::Error;
use std::str::FromStr;
use core::fmt;
use crate::tx_processor::TxKind::Deposit;

pub type BoxResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum TxKind {
    Withdrawal,
    Deposit,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Deserialize)]
pub struct Transaction {
    #[serde(rename(serialize = "type", deserialize = "type"))]
    tx_type: TxKind,
    client: u16,
    tx: u32,
    amount: String,
}

/// Turn a BigDecimal into a rounded &str
fn serialize_bigdec<S>(x: &BigDecimal, s: S) -> Result<S::Ok, S::Error> where S: Serializer {
    s.serialize_str(x.round(4i64).to_string().as_str())
}

#[derive(Debug, Serialize)]
pub struct Account {
    client: u16,
    #[serde(serialize_with = "serialize_bigdec")]
    available: BigDecimal,
    #[serde(serialize_with = "serialize_bigdec")]
    held: BigDecimal,
    #[serde(serialize_with = "serialize_bigdec")]
    total: BigDecimal,
    locked: bool,
    #[serde(skip_serializing)]
    disputed: HashSet<u32>,
}

impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}, {}, {}, {}",
               self.available.to_string().as_str(),
               self.held.to_string().as_str(),
               self.total.to_string().as_str(),
               self.locked)
    }
}

pub struct TxProcessor {
    accounts: HashMap<u16, Account>,
    transactions: HashMap<u32, Transaction>,
}

fn get_bigdec(str: String) -> BoxResult<BigDecimal> {
    Ok(BigDecimal::from_str(str.as_str())?)
}

impl TxProcessor {
    pub fn new() -> Self {
        TxProcessor {
            accounts: HashMap::new(),
            transactions: HashMap::new(),
        }
    }

    pub fn get_accounts(&self) -> &HashMap<u16, Account> {
        &self.accounts
    }

    pub fn process_tx(&mut self, tx: Transaction) -> BoxResult<()> {
        match tx.tx_type { // make sure amount is defined and non-negative for types with amount
            TxKind::Withdrawal | TxKind::Deposit => {
                if BigDecimal::from_str(tx.amount.as_str())? < BigDecimal::zero() {
                    return Ok(()); // ignore records with negative amounts
                }
            },
            _ => {},
        }

        match tx.tx_type {
            TxKind::Withdrawal => self.withdrawal(tx),
            TxKind::Deposit => self.deposit(tx),
            TxKind::Dispute => self.dispute(tx),
            TxKind::Resolve => self.resolve(tx),
            TxKind::Chargeback => self.chargeback(tx),
        }
    }

    /// Process a deposit tx by increasing account's balance
    fn deposit(&mut self, tx: Transaction) -> BoxResult<()> {
        let account = self.get_account(tx.client);
        if account.locked {
            return Ok(());
        }
        account.available += BigDecimal::from_str(tx.amount.as_str())?;
        account.total += BigDecimal::from_str(tx.amount.as_str())?;
        self.transactions.insert(tx.tx, tx);
        Ok(())
    }

    /// Process a withdrawal tx by decreasing account balance
    fn withdrawal(&mut self, tx: Transaction) -> BoxResult<()> {
        let account = self.get_account(tx.client);
        if account.locked {
            return Ok(());
        }
        let tx_amount = BigDecimal::from_str(tx.amount.as_str())?;
        if account.available >= tx_amount {
            account.available -= BigDecimal::from_str(tx.amount.as_str())?;
            account.total -= BigDecimal::from_str(tx.amount.as_str())?;
        }
        self.transactions.insert(tx.tx, tx);
        Ok(())
    }

    /// File a dispute over a deposit transaction
    fn dispute(&mut self, tx: Transaction) -> BoxResult<()> {
        if !self.is_tx_valid(tx.client, tx.tx) || !self.is_deposit(tx.tx) {
            return Ok(())
        }
        let disputed_amount = match self.get_tx_amount(tx.tx)? {
            Some(amount) => amount,
            None => return Ok(()),
        };
        let account = self.get_account(tx.client);
        if account.disputed.contains(&tx.tx) {
            return Ok(()) // already being disputed, nothing to do
        }
        account.disputed.insert(tx.tx);
        account.held += &disputed_amount;
        account.available -= disputed_amount;
        Ok(())
    }

    /// Resolve a disputed transaction by withdrawing the dispute
    fn resolve(&mut self, tx: Transaction) -> BoxResult<()> {
        if !self.is_tx_valid(tx.client, tx.tx) {
            return Ok(());
        }
        let disputed_amount = match self.get_tx_amount(tx.tx)? {
            Some(amount) => amount,
            None => return Ok(()),
        };
        let account = self.get_account(tx.client);
        if !account.disputed.contains(&tx.tx) {
            return Ok(()); // ignoring trying to resolve undisputed tx
        }
        account.disputed.remove(&tx.tx);
        account.held -= &disputed_amount;
        account.available += &disputed_amount;
        Ok(())
    }

    /// Chargeback for a disputed deposit transaction
    fn chargeback(&mut self, tx: Transaction) -> BoxResult<()> {
        if !self.is_tx_valid(tx.client, tx.tx) {
            return Ok(());
        }
        let disputed_amount = match self.get_tx_amount(tx.tx)? {
            Some(amount) => amount,
            None => return Ok(()),
        };
        let mut account = self.get_account(tx.client);
        if account.locked {
            return Ok(());
        }
        if !account.disputed.contains(&tx.tx) {
            return Ok(()); // ignoring trying to resolve undisputed tx
        }
        account.locked = true;
        account.disputed.remove(&tx.tx);
        account.held -= &disputed_amount;
        account.total -= &disputed_amount;
        Ok(())
    }

    /// Validate that reference transaction exists and that its client is the same as the client of
    /// the current transactions
    fn is_tx_valid(&self, client: u16, ref_tx: u32) -> bool {
        match self.transactions.get(&ref_tx) {
            Some(tx) => tx.client == client,
            None => false
        }
    }

    fn is_deposit(&self, tx_id: u32) -> bool {
        match self.transactions.get(&tx_id) {
            Some(tx) => tx.tx_type == Deposit,
            None => false
        }
    }

    fn get_tx_amount(&self, tx_id: u32) -> BoxResult<Option<BigDecimal>> {
        let tx = self.transactions.get(&tx_id);
        match tx {
            Some(tx) => Ok(Some(get_bigdec(tx.amount.clone())?)),
            None => Ok(None),
        }
    }

    /// Get an existing account or create an empty account
    fn get_account(&mut self, acc_id: u16) -> &mut Account {
        self.accounts.entry(acc_id).or_insert(Account {
            client: acc_id,
            available: BigDecimal::zero(),
            held: BigDecimal::zero(),
            total: BigDecimal::zero(),
            disputed: HashSet::new(),
            locked: false,
        })
    }
}
