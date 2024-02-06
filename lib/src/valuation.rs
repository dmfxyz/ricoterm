use std::cmp::max;

use ethers::{providers::Middleware, types::{Address, H256, U256, U512}};

use crate::{ddso::{feedbase::Feedbase, nfpm::NPFM, uniwrapper::UniWrapper, vat::{Vat, RU256}}, math::units};

pub struct Valuer<'a, T: Middleware + Clone> {
    pub npfm: &'a NPFM<T>,
    pub vat: &'a Vat<T>,
    pub feedbase: &'a Feedbase<T>,
    pub uniwrapper: &'a UniWrapper<T>,
}

impl <'a, T: Middleware + Clone> Valuer<'a, T> {
    pub fn new(npfm: &'a NPFM<T>, vat: &'a Vat<T>, feedbase: &'a Feedbase<T>, uniwrapper: &'a UniWrapper<T>) -> Self {
        Self {
            npfm: npfm,
            vat: vat,
            feedbase: feedbase,
            uniwrapper: uniwrapper,
        }
    }

    pub async fn value_uni_nft(
        &self,
        token_id: &U256,
    ) -> U256 {
        let position = self.npfm.positions(*token_id).await;
        let token_0_xs = {
            let mut bytes = [0u8; 32];
            bytes[0..20].copy_from_slice(position.token0.as_bytes());
            H256::from(bytes)
        };
        let t0_info: (Address, H256, U256) = (
            Address::from_slice(
                &self.vat.geth::<H256>(":uninft", "src", vec![token_0_xs])
                    .await
                    .as_bytes()[0..20],
            ),
            self.vat.geth(":uninft", "tag", vec![token_0_xs]).await,
            self.vat.geth::<RU256>(":uninft", "liqr", vec![token_0_xs])
                .await
                .into(),
        );
    
        let token_1_xs = {
            let mut bytes = [0u8; 32];
            bytes[0..20].copy_from_slice(position.token1.as_bytes());
            H256::from(bytes) // Con
        };
        let t1_info: (Address, H256, U256) = (
            Address::from_slice(
                &self.vat.geth::<H256>(":uninft", "src", vec![token_1_xs])
                    .await
                    .as_bytes()[0..20],
            ),
            self.vat.geth(":uninft", "tag", vec![token_1_xs]).await,
            self.vat.geth::<RU256>(":uninft", "liqr", vec![token_1_xs])
                .await
                .into(),
        );
    
        let t1_price_256: U256 =
            U256::from_big_endian(self.feedbase.pull(t1_info.0, t1_info.1).await.0.as_bytes());
        let t0_price_256: U256 =
            U256::from_big_endian(self.feedbase.pull(t0_info.0, t0_info.1).await.0.as_bytes());
        let t1_price: U512 = t1_price_256.try_into().unwrap();
        let t0_price: U512 = t0_price_256.try_into().unwrap();
        let scaled_t1_price: U512 = t1_price * U512::from(units::new().X96);
        let scaled_ration = scaled_t1_price * U512::from(units::new().X96) / t0_price;
        let price_256 = U256::try_from(scaled_ration.integer_sqrt()).unwrap();
        let total = self.uniwrapper.total(self.npfm.address, *token_id, price_256).await;
        let liqr = max(t0_info.2, t1_info.2);
        let value: U256 = (total.0 * t0_price_256 + total.1 * t1_price_256) / liqr;
        value
    }

    pub async fn value_gem(
        &self,
        gem: &str,
        ink: &U256,
    ) -> U256 {
        let liqr: U256 = self.vat.geth::<RU256>(gem, "liqr", Vec::new()).await.into();
        let src: Address = Address::from_slice(
            &self.vat.geth::<H256>(gem, "src", Vec::new()).await.as_bytes()[0..20],
        );
        let tag: H256 = self.vat.geth::<H256>(gem, "tag", Vec::new()).await;
        let rfeed: H256 = self.feedbase.pull(src, tag).await.0;
        let feed: U256 = U256::from_big_endian(rfeed.as_bytes());
        feed * ink / liqr
    }
}