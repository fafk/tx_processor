### A simple transaction processor utility

Takes in a bunch of transactions of users and computes the final state for all users. 

#### Tests
`$ cargo build && cargo test`

#### Usage

`cargo run -- transactions.csv > accounts.csv`

Where `transaction.csv` is the input containing a batch of transaction to process. See the contents
of the `test_data` folder for an example.

Output goes to standard output. ` > accounts.csv` in this example redirect it to a file.

#### Highlights

* BigDecimal - important to deal with floating point eccentricities (paramount when dealing with money)
* Precision - 4 places after decimal point (gets rounded depending on the 5th digit after zero)
* Rejects negative amounts
* If deposit/resolve/chargeback's client is different from the client of the referred tx, such tx
is considered erroneous and doesn't get processed 
* Frozen/locked account never get unlocked; presumably needs to be inspected by a human agent
* Deposits, withdrawals and chargebacks are not possible for frozen accounts, but it's possible
to file a dispute and resolve that dispute
* I/O, parsing and other errors get propagated and printed out and the program exits
* It's possible to end up with negative balance (deposit -> withdraw -> dispute [-> chargeback]);
such person is considered to owe money to the system owner
* Unit tests in `main.rs` _and_ integration tests in `tests/`
* Disputing an already disputed tx does nothing 
* It's not possible to dispute a withdrawal as it's not compatible with the specs

#### Things to improve

* Prettify errors 
