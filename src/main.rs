/// A utility that processes a batch of transactions and outputs the final state of accounts
/// that were affected by these transactions.
///
/// Usage: cargo run -- transactions.csv > accounts.csv
///
use std::{env, io};
use csv::Trim;
use crate::tx_processor::{Transaction, TxProcessor, BoxResult};
use std::io::Write;

mod tx_processor;

fn main() -> BoxResult<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Unexpected number of arguments: {}. Provide exactly 1 argument", args.len() - 1);
        println!("\tUsage: cargo run -- transactions.csv > accounts.csv");
        std::process::exit(exitcode::USAGE);
    }

    let processed = run_processing(args.get(1).unwrap())?;
    print_serialized(processed)?;

    return Ok(());
}

/// Continuously read from file, parse lines to structs and send it to tx processor
fn run_processing(path: &str) -> BoxResult<TxProcessor> {
    let mut reader = csv::ReaderBuilder::new().trim(Trim::All).from_path(path)?;
    let mut tx_processor = TxProcessor::new();

    for result in reader.deserialize() {
        let tx: Transaction = result?; // parsing fails if a record is malformed
        tx_processor.process_tx(tx)?;
    }

    return Ok(tx_processor);
}

/// Traverse map with accounts and print them as a cvs
fn print_serialized(tx_processor: TxProcessor) -> BoxResult<()> {
    // csv/serde lib throws an error when trying to serialize with prepended headers
    io::stdout().write_all(b"client,available,held,total,locked\n")?;

    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false).from_writer(io::stdout());

    for (_i, account) in tx_processor.get_accounts() {
        wtr.serialize(account)?;
    }

    wtr.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::run_processing;

    #[test]
    fn acc_two_ignores_invalid_withdrawal() {
        let res = run_processing("test_data/001.csv").unwrap();
        let accounts = res.get_accounts();
        assert_eq!(2, accounts.len());
        assert_eq!("1.5, 0, 1.5, false", accounts.get(&1).unwrap().to_string());
        assert_eq!("2.0, 0, 2.0, false", accounts.get(&2).unwrap().to_string());
    }

    #[test]
    fn dispute() {
        let res = run_processing("test_data/002.csv").unwrap();
        let accounts = res.get_accounts();
        assert_eq!(2, accounts.len());
        assert_eq!("1.0, 2.0, 3.0, false", accounts.get(&1).unwrap().to_string());
        assert_eq!("2.0, 0, 2.0, false", accounts.get(&2).unwrap().to_string());
    }

    #[test]
    fn dispute_for_mismatching_client() {
        let res = run_processing("test_data/003.csv").unwrap();
        let accounts = res.get_accounts();
        assert_eq!(2, accounts.len());
        assert_eq!("1.0, 0, 1.0, false", accounts.get(&1).unwrap().to_string());
        assert_eq!("1.0, 0, 1.0, false", accounts.get(&2).unwrap().to_string());
    }

    #[test]
    fn dispute_to_zero_with_with_chargeback() {
        let res = run_processing("test_data/005.csv").unwrap();
        let accounts = res.get_accounts();
        assert_eq!(1, accounts.len());
        assert_eq!("-1.0, 0.0, -1.0, true", accounts.get(&1).unwrap().to_string());
    }

    #[test]
    fn dispute_with_chargeback() {
        let res = run_processing("test_data/006.csv").unwrap();
        let accounts = res.get_accounts();
        assert_eq!(1, accounts.len());
        assert_eq!("-1.0, 0, -1.0, true", accounts.get(&1).unwrap().to_string());
    }

    #[test]
    // the last deposit is supposed not to go through, because the acc is frozen
    fn dispute_with_chargeback_and_deposit() {
        let res = run_processing("test_data/007.csv").unwrap();
        let accounts = res.get_accounts();
        assert_eq!(1, accounts.len());
        assert_eq!("-1.0, 0, -1.0, true", accounts.get(&1).unwrap().to_string());
    }

    #[test]
    fn dispute_with_resolve() {
        let res = run_processing("test_data/008.csv").unwrap();
        let accounts = res.get_accounts();
        assert_eq!(1, accounts.len());
        assert_eq!("2.0, 0, 2.0, false", accounts.get(&1).unwrap().to_string());
    }

    #[test]
    fn many_acc_with_transfers() {
        let res = run_processing("test_data/009.csv").unwrap();
        let accounts = res.get_accounts();
        assert_eq!(3, accounts.len());
        assert_eq!("17.25, 0, 17.25, false", accounts.get(&1).unwrap().to_string());
        assert_eq!("9.80, 0, 9.80, false", accounts.get(&2).unwrap().to_string());
        assert_eq!("33.123456, 0, 33.123456, false", accounts.get(&3).unwrap().to_string());
    }

    #[test]
    fn multiple_disputes_of_one_tx() {
        let res = run_processing("test_data/010.csv").unwrap();
        let accounts = res.get_accounts();
        assert_eq!(1, accounts.len());
        assert_eq!("0.0, 1.0, 1.0, false", accounts.get(&1).unwrap().to_string());
    }

    #[test]
    fn withdrawing_more_than_available() {
        let res = run_processing("test_data/011.csv").unwrap();
        let accounts = res.get_accounts();
        assert_eq!(1, accounts.len());
        assert_eq!("1.0, 0, 1.0, false", accounts.get(&1).unwrap().to_string());
    }

    #[test]
    fn deposits_withdrawals_disputes_and_resolves() {
        let res = run_processing("test_data/012.csv").unwrap();
        let accounts = res.get_accounts();
        assert_eq!(3, accounts.len());
        assert_eq!("17.25, 0.0, 17.25, false", accounts.get(&1).unwrap().to_string());
        assert_eq!("9.80, 0.0, 9.80, false", accounts.get(&2).unwrap().to_string());
        assert_eq!("33.123456, 0.000000, 33.123456, false", accounts.get(&3).unwrap().to_string());
    }
}
