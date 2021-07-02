//! Easy to use utilities for confirmations.

use crate::{
    api::{Eth, EthFilter, Namespace},
    error,
    types::{Bytes, TransactionReceipt, TransactionRequest, H256, U64},
    Transport,
};
use futures::{Future, StreamExt};
use std::time::Duration;

/// Checks whether an event has been confirmed.
pub trait ConfirmationCheck {
    /// Future resolved when is known whether an event has been confirmed.
    type Check: Future<Output = error::Result<Option<U64>>>;

    /// Should be called to get future which resolves when confirmation state is known.
    fn check(&self) -> Self::Check;
}

impl<F, T> ConfirmationCheck for F
where
    F: Fn() -> T,
    T: Future<Output = error::Result<Option<U64>>>,
{
    type Check = T;

    fn check(&self) -> Self::Check {
        (*self)()
    }
}

/// Should be used to wait for confirmations
pub async fn wait_for_confirmations<T, V, F>(
    eth: Eth<T>,
    eth_filter: EthFilter<T>,
    poll_interval: Duration,
    confirmations: usize,
    check: V,
) -> error::Result<()>
where
    T: Transport,
    V: ConfirmationCheck<Check = F>,
    F: Future<Output = error::Result<Option<U64>>>,
{
    let filter = eth_filter.create_blocks_filter().await?;
    // TODO #396: The stream should have additional checks.
    // * We should not continue calling next on a stream that has completed (has returned None). We expect this to never
    //   happen for the blocks filter but to be safe we should handle this case for example by `fuse`ing the stream or
    //   erroring when it does complete.
    // * We do not handle the case where the stream returns an error which means we are wrongly counting it as a
    //   confirmation.
    let filter_stream = filter.stream(poll_interval).skip(confirmations);
    futures::pin_mut!(filter_stream);
    loop {
        let _ = filter_stream.next().await;
        if let Some(confirmation_block_number) = check.check().await? {
            let block_number = eth.block_number().await?;
            if confirmation_block_number.low_u64() + confirmations as u64 <= block_number.low_u64() {
                return Ok(());
            }
        }
    }
}

async fn transaction_receipt_block_number_check<T: Transport>(eth: &Eth<T>, hash: H256) -> error::Result<Option<U64>> {
    let receipt = eth.transaction_receipt(hash).await?;
    Ok(receipt.and_then(|receipt| receipt.block_number))
}

async fn send_transaction_with_confirmation_<T: Transport>(
    hash: H256,
    transport: T,
    poll_interval: Duration,
    confirmations: usize,
) -> error::Result<TransactionReceipt> {
    let eth = Eth::new(transport.clone());
    if confirmations > 0 {
        let confirmation_check = || transaction_receipt_block_number_check(&eth, hash);
        let eth_filter = EthFilter::new(transport.clone());
        let eth = eth.clone();
        wait_for_confirmations(eth, eth_filter, poll_interval, confirmations, confirmation_check).await?;
    }
    // TODO #397: We should remove this `expect`. No matter what happens inside the node, this shouldn't be a panic.
    let receipt = eth
        .transaction_receipt(hash)
        .await?
        .expect("receipt can't be null after wait for confirmations; qed");
    Ok(receipt)
}

/// Sends transaction and returns future resolved after transaction is confirmed
pub async fn send_transaction_with_confirmation<T>(
    transport: T,
    tx: TransactionRequest,
    poll_interval: Duration,
    confirmations: usize,
) -> error::Result<TransactionReceipt>
where
    T: Transport,
{
    let hash = Eth::new(&transport).send_transaction(tx).await?;
    send_transaction_with_confirmation_(hash, transport, poll_interval, confirmations).await
}

/// Sends raw transaction and returns future resolved after transaction is confirmed
pub async fn send_raw_transaction_with_confirmation<T>(
    transport: T,
    tx: Bytes,
    poll_interval: Duration,
    confirmations: usize,
) -> error::Result<TransactionReceipt>
where
    T: Transport,
{
    let hash = Eth::new(&transport).send_raw_transaction(tx).await?;
    send_transaction_with_confirmation_(hash, transport, poll_interval, confirmations).await
}

#[cfg(test)]
mod tests {
    use super::send_transaction_with_confirmation;
    use crate::{
        rpc::Value,
        transports::test::TestTransport,
        types::{Address, TransactionReceipt, TransactionRequest, H256, U64},
    };
    use serde_json::json;
    use std::time::Duration;

    #[test]
    fn test_send_transaction_with_confirmation() {
        let mut transport = TestTransport::default();
        let confirmations = 3;
        let transaction_request = TransactionRequest {
            from: Address::from_low_u64_be(0x123),
            to: Some(Address::from_low_u64_be(0x123)),
            gas: None,
            gas_price: Some(1.into()),
            value: Some(1.into()),
            data: None,
            nonce: None,
            condition: None,
            transaction_type: None,
            access_list: None,
        };

        let transaction_receipt = TransactionReceipt {
            transaction_hash: H256::zero(),
            transaction_index: U64::zero(),
            block_hash: Some(H256::zero()),
            block_number: Some(2.into()),
            from: Address::from_low_u64_be(0x123),
            to: Some(Address::from_low_u64_be(0x123)),
            cumulative_gas_used: 0.into(),
            gas_used: Some(0.into()),
            contract_address: None,
            logs: vec![],
            status: Some(1.into()),
            root: Some(H256::zero()),
            logs_bloom: Default::default(),
            transaction_type: None,
        };

        let poll_interval = Duration::from_secs(0);
        transport.add_response(Value::String(
            r#"0x0000000000000000000000000000000000000000000000000000000000000111"#.into(),
        ));
        transport.add_response(Value::String("0x123".into()));
        transport.add_response(Value::Array(vec![
            Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000456"#.into()),
            Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000457"#.into()),
        ]));
        transport.add_response(Value::Array(vec![Value::String(
            r#"0x0000000000000000000000000000000000000000000000000000000000000458"#.into(),
        )]));
        transport.add_response(Value::Array(vec![Value::String(
            r#"0x0000000000000000000000000000000000000000000000000000000000000459"#.into(),
        )]));
        transport.add_response(Value::Null);
        transport.add_response(Value::Array(vec![
            Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000460"#.into()),
            Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000461"#.into()),
        ]));
        transport.add_response(Value::Null);
        transport.add_response(json!(transaction_receipt));
        transport.add_response(Value::String("0x6".into()));
        transport.add_response(json!(transaction_receipt));
        transport.add_response(Value::Bool(true));

        let confirmation = {
            let future =
                send_transaction_with_confirmation(&transport, transaction_request, poll_interval, confirmations);
            futures::executor::block_on(future)
        };

        transport.assert_request("eth_sendTransaction", &[r#"{"from":"0x0000000000000000000000000000000000000123","gasPrice":"0x1","to":"0x0000000000000000000000000000000000000123","value":"0x1"}"#.into()]);
        transport.assert_request("eth_newBlockFilter", &[]);
        transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
        transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
        transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
        transport.assert_request(
            "eth_getTransactionReceipt",
            &[r#""0x0000000000000000000000000000000000000000000000000000000000000111""#.into()],
        );
        transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
        transport.assert_request(
            "eth_getTransactionReceipt",
            &[r#""0x0000000000000000000000000000000000000000000000000000000000000111""#.into()],
        );
        transport.assert_request(
            "eth_getTransactionReceipt",
            &[r#""0x0000000000000000000000000000000000000000000000000000000000000111""#.into()],
        );
        transport.assert_request("eth_blockNumber", &[]);
        transport.assert_request(
            "eth_getTransactionReceipt",
            &[r#""0x0000000000000000000000000000000000000000000000000000000000000111""#.into()],
        );
        transport.assert_no_more_requests();
        assert_eq!(confirmation, Ok(transaction_receipt));
    }
}
