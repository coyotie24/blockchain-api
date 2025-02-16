use {
    super::HistoryProvider,
    crate::{
        error::{RpcError, RpcResult},
        handlers::history::{
            HistoryQueryParams,
            HistoryResponseBody,
            HistoryTransaction,
            HistoryTransactionFungibleInfo,
            HistoryTransactionMetadata,
            HistoryTransactionTransfer,
            HistoryTransactionTransferQuantity,
        },
        utils::crypto::string_chain_id_to_caip2_format,
    },
    async_trait::async_trait,
    hyper::Client,
    hyper_tls::HttpsConnector,
    serde::{Deserialize, Serialize},
    tracing::log::error,
    url::Url,
};

#[derive(Debug)]
pub struct CoinbaseProvider {
    pub api_key: String,
    pub app_id: String,
    pub http_client: Client<HttpsConnector<hyper::client::HttpConnector>>,
}

impl CoinbaseProvider {
    pub fn new(api_key: String, app_id: String) -> Self {
        let http_client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());
        Self {
            api_key,
            app_id,
            http_client,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CoinbaseResponseBody {
    pub transactions: Vec<CoinbaseTransaction>,
    pub next_page_key: Option<String>,
    pub total_count: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CoinbaseTransaction {
    pub status: String,
    pub transaction_id: String,
    pub tx_hash: String,
    pub created_at: String,
    pub purchase_network: String,
    pub purchase_amount: CoinbasePurchaseAmount,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CoinbasePurchaseAmount {
    pub value: String,
    pub currency: String,
}

#[async_trait]
impl HistoryProvider for CoinbaseProvider {
    #[tracing::instrument(skip(self, params), fields(provider = "Coinbase"))]
    async fn get_transactions(
        &self,
        address: String,
        params: HistoryQueryParams,
        http_client: reqwest::Client,
    ) -> RpcResult<HistoryResponseBody> {
        let base = format!(
            "https://pay.coinbase.com/api/v1/buy/user/{}/transactions",
            &address
        );

        let mut url = Url::parse(&base).map_err(|_| RpcError::HistoryParseCursorError)?;
        url.query_pairs_mut().append_pair("page_size", "50");

        if let Some(cursor) = params.cursor {
            url.query_pairs_mut().append_pair("page_key", &cursor);
        }

        let response = http_client
            .get(url)
            .header("Content-Type", "application/json")
            .header("CBPAY-APP-ID", self.app_id.clone())
            .header("CBPAY-API-KEY", self.api_key.clone())
            .send()
            .await?;

        if response.status() != reqwest::StatusCode::OK {
            error!(
                "Error on Coinbase transactions response. Status is not OK: {:?}",
                response.status(),
            );
            return Err(RpcError::TransactionProviderError);
        }

        let body = response.json::<CoinbaseResponseBody>().await?;

        let transactions = body
            .transactions
            .into_iter()
            .map(|f| HistoryTransaction {
                id: f.transaction_id,
                metadata: HistoryTransactionMetadata {
                    operation_type: "buy".to_string(),
                    hash: f.tx_hash,
                    mined_at: f.created_at,
                    nonce: 1, // TODO: get nonce from somewhere
                    sent_from: "Coinbase".to_string(),
                    sent_to: address.clone(),
                    status: f.status,
                    application: None,
                    chain: string_chain_id_to_caip2_format(&f.purchase_network).ok(),
                },
                transfers: Some(vec![HistoryTransactionTransfer {
                    fungible_info: Some(HistoryTransactionFungibleInfo {
                        name: Some(f.purchase_amount.currency.clone()),
                        symbol: Some(f.purchase_amount.currency),
                        icon: None,
                    }),
                    direction: "in".to_string(),
                    quantity: HistoryTransactionTransferQuantity {
                        numeric: f.purchase_amount.value,
                    },
                    nft_info: None,
                    value: None,
                    price: None,
                }]),
            })
            .collect();

        Ok(HistoryResponseBody {
            data: transactions,
            next: body.next_page_key,
        })
    }
}
