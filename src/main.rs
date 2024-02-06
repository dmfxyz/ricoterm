mod config;
mod urn;
use chrono::NaiveDateTime;
use crossterm::{
    event::{self, DisableMouseCapture, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ethers::prelude::*;
use ethers::types::U256;
use ricolib::{feedbase::Feedbase, math::units, nfpm::NPFM, uniwrapper::UniWrapper, vat::*};
use std::{
    cmp::max,
    collections::HashMap,
    convert::TryFrom,
    io,
    sync::mpsc,
    sync::{Arc, Mutex},
    thread,
};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use urn::UrnData;

use tui::style::Modifier;
use tui::style::Style;

use ricolib::vox::Vox;

async fn value_uni_nft<T: Middleware + Clone>(
    token_id: &U256,
    npfm: &NPFM<T>,
    vat: &Vat<T>,
    feedbase: &Feedbase<T>,
    uniwrapper: &UniWrapper<T>,
) -> U256 {
    let position = npfm.positions(*token_id).await;
    let token_0_xs = {
        let mut bytes = [0u8; 32]; 
        bytes[0..20].copy_from_slice(position.token0.as_bytes());
        H256::from(bytes)
    };
    let t0_info: (Address, H256, U256) = (
        Address::from_slice(
            &vat.geth::<H256>(":uninft", "src", vec![token_0_xs])
                .await
                .as_bytes()[0..20],
        ),
        vat.geth(":uninft", "tag", vec![token_0_xs]).await,
        vat.geth::<RU256>(":uninft", "liqr", vec![token_0_xs])
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
            &vat.geth::<H256>(":uninft", "src", vec![token_1_xs])
                .await
                .as_bytes()[0..20],
        ),
        vat.geth(":uninft", "tag", vec![token_1_xs]).await,
        vat.geth::<RU256>(":uninft", "liqr", vec![token_1_xs])
            .await
            .into(),
    );

    let t1_price_256: U256 =
        U256::from_big_endian(feedbase.pull(t1_info.0, t1_info.1).await.0.as_bytes());
    let t0_price_256: U256 =
        U256::from_big_endian(feedbase.pull(t0_info.0, t0_info.1).await.0.as_bytes());
    let t1_price: U512 = t1_price_256.try_into().unwrap();
    let t0_price: U512 = t0_price_256.try_into().unwrap();
    let scaled_t1_price: U512 = t1_price * U512::from(units::new().X96);
    let scaled_ration = scaled_t1_price * U512::from(units::new().X96) / t0_price;
    let price_256 = U256::try_from(scaled_ration.integer_sqrt()).unwrap();
    let total = uniwrapper.total(npfm.address, *token_id, price_256).await;
    let liqr = max(t0_info.2, t1_info.2);
    let value: U256 = (total.0 * t0_price_256 + total.1 * t1_price_256) / liqr;
    value
}

async fn fetch_all_urn_data_for_ilk<T: Middleware + Clone>(
    ilk: &str,
    vat: &Vat<T>,
    feedbase: &Feedbase<T>,
    npfm: &NPFM<T>,
    uniwrapper: &UniWrapper<T>,
    wallet_address: Address,
) -> UrnData {
    let par = vat.par().await;
    let ink: U256 = match ilk.eq(":uninft") {
        false => vat
            .ink(ilk, wallet_address)
            .await
            .get(0)
            .unwrap()
            .to_owned(),
        true => {
            let ink = vat.ink(ilk, wallet_address).await;
            let mut total_ink = U256::zero();
            for token in ink {
                total_ink += value_uni_nft(&token, npfm, vat, feedbase, uniwrapper).await;
            }
            total_ink
        }
    };
    let art: U256 = vat.urns(ilk, wallet_address).await;
    let ililk: Ilk = vat.ilks(ilk).await;
    let loan = art * ililk.rack * par / U256::from(10_u128.pow(27)) / U256::from(10_u128.pow(27));
    let value = match ilk {
        ":uninft" => ink,
        _ => {
            let liqr: U256 = vat.geth::<RU256>(ilk, "liqr", Vec::new()).await.into();
            let src: Address = Address::from_slice(
                &vat.geth::<H256>(ilk, "src", Vec::new()).await.as_bytes()[0..20],
            );
            let tag: H256 = vat.geth::<H256>(ilk, "tag", Vec::new()).await;
            let rfeed: H256 = feedbase.pull(src, tag).await.0;
            let feed: U256 = U256::from_big_endian(rfeed.as_bytes());
            feed * ink / liqr
        }
    };

    let safety = match loan.cmp(&U256::zero()) {
        std::cmp::Ordering::Equal => 0.0,
        _ => (U256::from(10_u128.pow(9)) * value / loan).as_u128() as f64 / 10_u128.pow(9) as f64,
    };

    UrnData {
        ink_name: String::from(ilk),
        ink,
        art,
        loan,
        value,
        safety,
    }
}

#[allow(clippy::too_many_arguments)]
async fn fetch_data<T: Middleware + Clone>(
    provider: Arc<Provider<Http>>,
    vat: &Vat<T>,
    vox: &Vox<T>,
    ilks: &Vec<String>,
    feedbase: &Feedbase<T>,
    npfm: &NPFM<T>,
    uniwrapper: &UniWrapper<T>,
    wallet_address: Address,
    active_ilk: &Arc<Mutex<Vec<String>>>,
) -> Result<(Vec<UrnData>, U256, U256, U64, NaiveDateTime, Vec<Ilk>), Box<dyn std::error::Error>> {
    let mut urn_data = Vec::new();
    for ilk in ilks {
        urn_data.push(
            fetch_all_urn_data_for_ilk(ilk, vat, feedbase, npfm, uniwrapper, wallet_address).await,
        );
    }
    // // get current block number from the provider
    let par = vat.par().await;
    let mar = U256::from_big_endian(
        feedbase
            .pull(vox.tip().await.0, vox.tip().await.1)
            .await
            .0
            .as_bytes(),
    );
    let block = provider.get_block_number().await?;
    let last_refreshed = provider.get_block(block).await?.unwrap().timestamp;
    let last_refreshed_as_time =
        chrono::NaiveDateTime::from_timestamp_opt(last_refreshed.as_u64() as i64, 0).unwrap();

    let active_ilk = { active_ilk.lock().unwrap().clone() };
    let mut ilk_data = Vec::<Ilk>::new();
    for ilk in active_ilk.iter() {
        let ilk_info = vat.ilks(ilk.as_str()).await;
        ilk_data.push(ilk_info);
    }

    Ok((urn_data, par, mar, block, last_refreshed_as_time, ilk_data))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::read_config("./term.toml")?;
    let ilk_help_message = format!("{:?}", &config.ilks.key_mappings);
    let mut live_ilks_key_char: HashMap<KeyCode, String> = HashMap::new();
    for (key, value) in config.ilks.key_mappings.into_iter() {
        live_ilks_key_char.insert(KeyCode::Char(key), value.to_string());
    }
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let provider = Arc::new(Provider::<Http>::try_from(config.rpc.arb_rpc_url.as_str())?);
    let vat = Vat::new(&provider, config.rico.diamond.parse()?);
    let vox = Vox::new(&provider, config.rico.diamond.parse()?);
    let fb = Feedbase::new(&provider, config.rico.feedbase.parse()?);
    let npfm = NPFM::new(&provider, config.rico.npfm.parse()?);
    let uniwrapper = UniWrapper::new(&provider, config.rico.uniwrapper.parse()?);
    let wallet_address: Address = config.urns.user_address.parse()?;
    let units = units::new();
    let (tx, rx) = mpsc::channel();
    let mut empty_urn_vec = Vec::<UrnData>::new();
    for urn in config.urns.ilks.iter() {
        empty_urn_vec.push(UrnData {
            ink_name: urn.to_string(),
            ..Default::default()
        })
    }
    let data = Arc::new(Mutex::new((
        empty_urn_vec,
        U256::zero(),
        U256::zero(),
        U64::zero(),
        NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
        Vec::<Ilk>::new(),
    )));

    // Spawn background task for fetching data
    let provider_clone = provider.clone();
    let data_clone = data.clone();
    let ilk_clone = config.urns.ilks.clone();
    let active_ilk = Arc::new(Mutex::new(Vec::<String>::new()));
    let active_ilk_clone = active_ilk.clone();
    let mut setting_active: bool = false;
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            loop {
                match fetch_data(
                    provider_clone.clone(),
                    &vat,
                    &vox,
                    &ilk_clone,
                    &fb,
                    &npfm,
                    &uniwrapper,
                    wallet_address,
                    &active_ilk_clone,
                )
                .await
                {
                    Ok((urn_data, par, mar, block, timestamp, ilk)) => {
                        let mut data = data_clone.lock().unwrap();
                        *data = (urn_data, par, mar, block, timestamp, ilk);
                        tx.send(()).unwrap();
                    }
                    Err(e) => println!("Error fetching data: {}", e),
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });
    });

    // UI event loop
    loop {
        // Draw UI
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(5),      // Top / Hero
                        Constraint::Percentage(65), // Main display area for urns and active views
                        Constraint::Percentage(30), // Footer and help section
                    ]
                    .as_ref(),
                )
                .split(size);

            let top_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(50),
                        Constraint::Percentage(50),
                    ]
                    .as_ref(),
                )
                .split(chunks[0]);

            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(50), // urns
                        Constraint::Percentage(50), // active view / ilks
                    ]
                    .as_ref(),
                )
                .split(chunks[1]);

            // dynamically allocate space for each urn
            let display_space_per_urn =100 / config.urns.ilks.len() as u16;
            let urn_display_constraints = config.urns.ilks.iter().map(|_| Constraint::Percentage(display_space_per_urn)).collect::<Vec<Constraint>>();
            let urn_views = Layout::default()
                .direction(Direction::Vertical)
                .constraints(urn_display_constraints.as_ref())
                .split(main_chunks[0]);
            let active_view = &main_chunks[1];
            // Populate top section with title
            let title = Paragraph::new(match &config.urns.user_nickname {
                Some(nickname) => format!("{}'s urns", nickname),
                None => format!("{}'s urns", &config.urns.user_address)
            })
                .style(Style::default().add_modifier(Modifier::BOLD)); // Optional: Add styling as needed
            f.render_widget(title, top_chunks[0]);
            // Grab data from the data mutex and start populating
            let data = data.lock().unwrap();
            let active_ilk = active_ilk.lock().unwrap();
            let mar = data.2;
            let par = data.1;

            // Build mar/par view
            let written_text = match mar.cmp(&par) {
                std::cmp::Ordering::Greater => "mar > par, price rate is decreasing",
                std::cmp::Ordering::Less => "mar < par, price rate is increasingq",
                std::cmp::Ordering::Equal => "mar = par",
            };

            let marpar_text = format!(
                "par: {}\nmar: {}\n{}",
                data.1.as_u128() as f64 / 10_u128.pow(27) as f64,
                data.2.as_u128() as f64 / 10_u128.pow(27) as f64,
                written_text
            );
            let marpar_paragraph = Paragraph::new(marpar_text)
                .block(Block::default().title("mar/par").borders(Borders::ALL));
            f.render_widget(marpar_paragraph, top_chunks[1]);

            // build and render widget for each urn in the main view
            for (i, urn) in data.0.iter().enumerate() {
                let urn_text = format!(
                    "art: {}\n\tink: {} \nloan: {} \nvalue: {}\nsafety: {}",
                    urn.art.low_u64() as f64 / 10_u64.pow(18) as f64,
                    urn.ink.low_u64() as f64 / match urn.ink_name.as_str() {"usdc" => 10_u64.pow(6) as f64, _ => 10_u64.pow(18) as f64},
                    (urn.loan.as_u128() as f64 / 10_u64.pow(18) as f64),
                    (urn.value.as_u128() as f64 / 10_u64.pow(18) as f64),
                    urn.safety
                );
                let urn_paragraph = Paragraph::new(urn_text)
                    .block(Block::default().title(urn.ink_name.as_str()).borders(Borders::ALL));
                f.render_widget(urn_paragraph, urn_views[i]);
            }

            // populate active view, which is settings or ilk view
            let active_view_text = match active_ilk.len() > 0{
               true => {
                    let all_ilk_data = &data.5;

                    // now, instead, iterate over all active ilks and append their data to the active view
                    let formatted_str = active_ilk.iter().enumerate().map(|(index, ilk)| {
                        format!(
                            "ilk: {}\n{}",
                            ilk,
                            match all_ilk_data.get(index) {
                                Some(ilk) => {
                                    let time_since_update = data.4 - NaiveDateTime::from_timestamp_opt(ilk.rho.as_u128() as i64, 0).unwrap();
                                    let time_since_update_string = format!(
                                        "{} hours, {} minutes, {} seconds",
                                        time_since_update.num_seconds() / 3600,
                                        (time_since_update.num_seconds() % 3600) / 60,
                                        time_since_update.num_seconds() % 60
                                    );
                                    format!(
                                        "  tart: {}\n  rho: {} UTC ({} hours ago)\n  dust: {}\n  fee: {}%",
                                        (((ilk.tart * units.BLN) / units.WAD).as_u128() as f64 / units.BLN_F64),
                                        NaiveDateTime::from_timestamp_opt(ilk.rho.as_u128() as i64, 0).unwrap(),
                                        time_since_update_string,
                                        (((ilk.dust * units.BLN) / units.RAD).as_u128() as f64
                                            / units.BLN_F64),
                                        (((ilk.fee * units.WAD) / units.RAY).as_u128() as f64 / units.BLN_F64.powf(2.0) - 1_f64) * units.BANKYEAR * 100.0
                                    )
                                }
                                _ => "  Awaiting ilk data...".to_string(),
                            })
                    }).collect::<Vec<String>>().join("\n");
                    formatted_str
                },
                _ => match setting_active {
                    true => format!("settings:\nrpc_url: {}, \nwallet_address: {}\nilks: {}", config.rpc.arb_rpc_url, config.urns.user_address, config.urns.ilks.join(", ")),
                    false => "No active view".to_string(),
                }
            };

            let active_view_paragraph = Paragraph::new(active_view_text)
                .block(Block::default().title("active_view").borders(Borders::ALL));
            f.render_widget(active_view_paragraph, *active_view);

            let footer_spans = vec![
                Spans::from(vec![
                    Span::styled("Last Block: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(format!("{} ", data.3)),
                    Span::styled("Last Refreshed: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(format!("{} UTC ", data.4)),
                ]),
                Spans::from(vec![
                    Span::styled("global_controls: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("'q' to quit, 'p' to pop last ilk, 'c' to clear active view, 's' to view settings\n"),
                ]),
                Spans::from(vec![
                    Span::styled("ilk_shortcuts: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(ilk_help_message.as_str()),
                ]),
            ];
            // Converting the collection of Spans into a single Text object
            let footer_text = Text::from(footer_spans);
            let footer_paragraph = Paragraph::new(footer_text).block(
                Block::default()
                    .title("ricoterm info")
                    .borders(Borders::ALL),
            );
            f.render_widget(footer_paragraph, chunks[2]);
        })?;

        if rx.try_recv().is_ok() {
            // Data was refreshed
        } else if event::poll(std::time::Duration::from_millis(200))? {
            if let event::Event::Key(key) = event::read()? {
                match live_ilks_key_char.get(&key.code).cloned() {
                    Some(ilk) => {
                        let mut active_ilk = active_ilk.lock().unwrap();
                        if !active_ilk.contains(&ilk) {
                            active_ilk.push(ilk);
                        }
                        //data.lock().unwrap().5 = Vec::<Ilk>::new();
                    }
                    _ => match key.code {
                        KeyCode::Char('q') => {
                            break;
                        }
                        KeyCode::Char('c') => {
                            let mut active_ilk = active_ilk.lock().unwrap();
                            *active_ilk = Vec::<String>::new();
                            data.lock().unwrap().5 = Vec::<Ilk>::new();
                            setting_active = false;
                        }
                        KeyCode::Char('s') => {
                            setting_active = true;
                            data.lock().unwrap().5 = Vec::<Ilk>::new();
                            *active_ilk.lock().unwrap() = Vec::<String>::new();
                        }
                        KeyCode::Char('p') => {
                            active_ilk.lock().unwrap().pop();
                            data.lock().unwrap().5.pop();
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
