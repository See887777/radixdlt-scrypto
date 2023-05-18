use clap::Parser;
use radix_engine::types::*;
use transaction::builder::ManifestBuilder;
use radix_engine_interface::api::node_modules::metadata::{MetadataEntry, MetadataValue};
use crate::resim::*;

/// Create a fungible token with mutable supply
#[derive(Parser, Debug)]
pub struct NewTokenMutable {
    /// The minter resource address
    pub minter_badge: SimulatorResourceOrNonFungibleGlobalId,

    /// The symbol
    #[clap(long)]
    pub symbol: Option<String>,

    /// The name
    #[clap(long)]
    pub name: Option<String>,

    /// The description
    #[clap(long)]
    pub description: Option<String>,

    /// The website URL
    #[clap(long)]
    pub url: Option<String>,

    /// The ICON url
    #[clap(long)]
    pub icon_url: Option<String>,

    /// The network to use when outputting manifest, [simulator | adapanet | nebunet | mainnet]
    #[clap(short, long)]
    pub network: Option<String>,

    /// Output a transaction manifest without execution
    #[clap(short, long)]
    pub manifest: Option<PathBuf>,

    /// The private keys used for signing, separated by comma
    #[clap(short, long)]
    pub signing_keys: Option<String>,

    /// Turn on tracing
    #[clap(short, long)]
    pub trace: bool,
}

impl NewTokenMutable {
    pub fn run<O: std::io::Write>(&self, out: &mut O) -> Result<(), Error> {
        let mut metadata = BTreeMap::new();
        if let Some(symbol) = self.symbol.clone() {
            metadata.insert("symbol".to_string(), MetadataEntry::Value(MetadataValue::String(symbol)));
        }
        if let Some(name) = self.name.clone() {
            metadata.insert("name".to_string(), MetadataEntry::Value(MetadataValue::String(name)));
        }
        if let Some(description) = self.description.clone() {
            metadata.insert("description".to_string(), MetadataEntry::Value(MetadataValue::String(description)));
        }
        if let Some(url) = self.url.clone() {
            metadata.insert("url".to_string(), MetadataEntry::Value(MetadataValue::String(url)));
        }
        if let Some(icon_url) = self.icon_url.clone() {
            metadata.insert("icon_url".to_string(), MetadataEntry::Value(MetadataValue::String(icon_url)));
        };

        let manifest = ManifestBuilder::new()
            .lock_fee(FAUCET, 100.into())
            .new_token_mutable(metadata, self.minter_badge.clone().into())
            .build();
        handle_manifest(
            manifest,
            &self.signing_keys,
            &self.network,
            &self.manifest,
            self.trace,
            true,
            out,
        )
        .map(|_| ())
    }
}
