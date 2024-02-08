mod config;
mod monet;
mod urn;
use chrono::NaiveDateTime;
use crossterm::{
    event::{self, DisableMouseCapture, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ethers::prelude::*;
use ethers::types::U256;
use ricolib::{
    ddso::{
        events::{IntoNewPalm2Vec, NewPalm2, NEW_PALM_2_SIG},
        feedbase::Feedbase,
        nfpm::NPFM,
        uniwrapper::UniWrapper,
        vat::*,
        vox::*,
    },
    math::units,
    utils::{bytes32_to_string, string_to_bytes32},
    valuation::Valuer,
};
use std::{
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
    style::{self, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use urn::UrnData;

use tui::style::Modifier;

async fn fetch_all_urn_data_for_ilk<T: Middleware + Clone>(
    ilk: &str,
    vat: &Vat<T>,
    feedbase: &Feedbase<T>,
    npfm: &NPFM<T>,
    uniwrapper: &UniWrapper<T>,
    wallet_address: Address,
) -> UrnData {
    let units = units::new();
    let valuer = Valuer::new(npfm, vat, feedbase, uniwrapper);
    let par = vat.par().await;
    let mut ninks = Option::<Vec<U256>>::None;
    let ink: U256 = match ilk.eq(":uninft") {
        false => vat
            .ink(ilk, wallet_address)
            .await
            .get(0)
            .unwrap()
            .to_owned(),
        true => {
            let ink = vat.ink(ilk, wallet_address).await;
            let mut _ninks = Vec::<U256>::new();
            let mut total_ink = U256::zero();
            for token in ink {
                total_ink += valuer.value_uni_nft(&token).await;
                _ninks.push(token);
            }
            ninks = Some(_ninks);
            total_ink
        }
    };
    let art: U256 = vat.urns(ilk, wallet_address).await;
    let ililk: Ilk = vat.ilks(ilk).await;
    let seconds_since_last_drip: i64 = (chrono::Utc::now().naive_utc()
        - NaiveDateTime::from_timestamp_opt(ililk.rho.as_u128() as i64, 0).unwrap())
    .num_seconds();
    let syn_rack =
        (0..seconds_since_last_drip).fold(ililk.rack, |acc, _| acc * ililk.fee / units.RAY);
    let loan = art * syn_rack * par / units.RAY / units.RAY;
    let value = match ilk {
        ":uninft" => ink,
        _ => valuer.value_gem(ilk, &ink).await,
    };

    let safety = match loan.cmp(&U256::zero()) {
        std::cmp::Ordering::Equal => 0.0,
        _ => (units.BLN * value / loan).as_u128() as f64 / units.BLN_F64,
    };

    let debt = art * syn_rack * units.BLN / units.RAY / units.BLN;

    UrnData {
        ink_name: String::from(ilk),
        ink,
        art,
        debt,
        loan,
        value,
        safety,
        ninks,
    }
}

#[allow(clippy::too_many_arguments)]
async fn fetch_data<T: Middleware + Clone>(
    provider: Arc<Provider<Http>>,
    world: Arc<RicoWorld<T>>,
    state: &Arc<Mutex<State>>,
) -> Result<ChainData, Box<dyn std::error::Error>> {
    let mut urn_data = Vec::<UrnData>::new();
    let (urns, active_ilks, wallet_address, active_palm_2) = {
        let state = state.lock().unwrap();
        (
            state.urns.clone(),
            state.active_ilk.clone(),
            state.user_address,
            state.active_new_palm_2,
        )
    };

    for ilk in urns {
        urn_data.push(
            fetch_all_urn_data_for_ilk(
                &ilk,
                &world.vat,
                &world.feedbase,
                &world.npfm,
                &world.uniwrapper,
                wallet_address,
            )
            .await,
        );
    }
    // // get current block number from the provider
    let par = world.vat.par().await;
    let mar = U256::from_big_endian(
        world
            .feedbase
            .pull(world.vox.tip().await.0, world.vox.tip().await.1)
            .await
            .0
            .as_bytes(),
    );
    let block = provider.get_block_number().await?;
    let last_refreshed = provider.get_block(block).await?.unwrap().timestamp;
    let last_refreshed_as_time =
        chrono::NaiveDateTime::from_timestamp_opt(last_refreshed.as_u64() as i64, 0).unwrap();

    let mut ilk_data = Vec::<Ilk>::new();
    for ilk in active_ilks.iter() {
        let ilk_info = world.vat.ilks(ilk.as_str()).await;
        ilk_data.push(ilk_info);
    }
    let way = world.vox.way().await;
    let tau = world.vox.tau().await;
    let how = world.vox.how().await;
    let xau = U256::from_big_endian(
        world
            .feedbase
            .pull(world.chainlink_address, string_to_bytes32("xau:usd"))
            .await
            .0
            .as_bytes(),
    );

    let mut logs = match active_palm_2 {
        Some("art") => {
            let filter = Filter::new()
                .address(vec![world.vat.address])
                .topic0(*NEW_PALM_2_SIG)
                .topic1(string_to_bytes32("art"))
                .from_block(BlockNumber::Earliest)
                .to_block(block);

            provider.get_logs(&filter).await?
        }
        _ => Vec::new(),
    };

    logs.sort_by(|a, b| a.block_number.cmp(&b.block_number));
    logs.reverse();

    Ok(ChainData {
        urn_data,
        par,
        mar,
        block,
        last_refreshed: last_refreshed_as_time,
        ilks: ilk_data,
        way,
        tau,
        how,
        xau,
        logs: logs.into_new_palm2_vec(),
    })
}

pub struct ChainData {
    pub urn_data: Vec<UrnData>,
    pub par: U256,
    pub mar: U256,
    pub block: U64,
    pub last_refreshed: NaiveDateTime,
    pub ilks: Vec<Ilk>,
    pub way: U256,
    pub tau: U256,
    pub how: U256,
    pub xau: U256,
    pub logs: Vec<NewPalm2>,
}

pub struct RicoWorld<T: Middleware + Clone> {
    vat: Vat<T>,
    vox: Vox<T>,
    feedbase: Feedbase<T>,
    npfm: NPFM<T>,
    uniwrapper: UniWrapper<T>,
    chainlink_address: Address,
}

pub struct State {
    pub active_ilk: Vec<String>,
    pub urns: Vec<String>,
    pub user_address: Address,
    pub active_new_palm_2: Option<&'static str>,
    pub selected_menu_view: SelectedMenuView,
    pub selected_active_view: SelectedActiveView,
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::read_config("./term.toml")?;
    let ilk_help_message = format!("{:?}", &config.ilks.key_mappings);
    let mut live_ilks_key_char: HashMap<KeyCode, String> = HashMap::new();
    for (key, value) in config.ilks.key_mappings.clone().into_iter() {
        live_ilks_key_char.insert(KeyCode::Char(key), value.to_string());
    }
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let (tx, rx) = mpsc::channel();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let provider = Arc::new(Provider::<Http>::try_from(config.rpc.arb_rpc_url.as_str())?);
    let wallet_address: Address = config.urns.user_address.parse()?;

    let world = Arc::new(RicoWorld {
        vat: Vat::new(&provider, config.rico.diamond.parse()?),
        vox: Vox::new(&provider, config.rico.diamond.parse()?),
        feedbase: Feedbase::new(&provider, config.rico.feedbase.parse()?),
        npfm: NPFM::new(&provider, config.rico.npfm.parse()?),
        uniwrapper: UniWrapper::new(&provider, config.rico.uniwrapper.parse()?),
        chainlink_address: config.rico.chain_link_feed.parse()?,
    });

    let state = Arc::new(Mutex::new(State {
        active_ilk: Vec::<String>::new(),
        urns: config.urns.ilks.clone(),
        user_address: wallet_address,
        active_new_palm_2: None,
        selected_menu_view: SelectedMenuView::Urn,
        selected_active_view: SelectedActiveView::Clear,
    }));

    let mut empty_urn_vec = Vec::<UrnData>::new();
    for urn in config.urns.ilks.iter() {
        empty_urn_vec.push(UrnData {
            ink_name: urn.to_string(),
            ..Default::default()
        })
    }
    let data = Arc::new(Mutex::new(ChainData {
        urn_data: empty_urn_vec,
        par: U256::zero(),
        mar: U256::zero(),
        block: U64::zero(),
        last_refreshed: chrono::NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
        ilks: Vec::<Ilk>::new(),
        way: U256::zero(),
        tau: U256::zero(),
        how: U256::zero(),
        xau: U256::zero(),
        logs: Vec::<NewPalm2>::new(),
    }));

    // Spawn background task for fetching data
    let provider_clone = provider.clone();
    let data_clone = data.clone();
    let state_clone = state.clone();
    let mut setting_active: bool = false;
    let mut menu_index: i32 = -1;
    let mut selected_menu_view = SelectedMenuView::Urn;
    let mut selected_market_view = SelectedMarketView::MarAndPar;
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            loop {
                match fetch_data(provider_clone.clone(), Arc::clone(&world), &state_clone).await {
                    Ok(new_data) => {
                        let mut data = data_clone.lock().unwrap();
                        *data = new_data;
                        tx.send(()).unwrap();
                    }
                    Err(e) => println!("Error fetching data: {}", e),
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(config.rpc.refresh_seconds))
                    .await;
            }
        });
    });

    // UI event loop
    loop {
        // Draw UI
        terminal.draw(|f| {
            let size = f.size();
            let mut canvas = monet::TermCanvas::init(size);
            // Populate top section with title
            let title = Paragraph::new(match &config.urns.user_nickname {
                Some(nickname) => format!("{}'s urns", nickname),
                None => format!("{}'s urns", &config.urns.user_address),
            })
            .style(Style::default().add_modifier(Modifier::BOLD));
            f.render_widget(title, canvas.navbar);
            // Grab data from the data mutex and start populating
            let data = data.lock().unwrap();
            let state = state.lock().unwrap();
            // Build mar/par view

            let marpar_paragraph = match selected_market_view {
                SelectedMarketView::MarAndPar => monet::paint_marpar(
                    data.mar,
                    data.par,
                    data.way,
                    data.tau,
                    data.how,
                    data.last_refreshed,
                ),
                SelectedMarketView::DollarConversion => {
                    monet::paint_pricing_screen(data.mar, data.par, data.xau)
                }
            };

            f.render_widget(marpar_paragraph, canvas.right_main_pane.market_view);

            // build and render widget for each urn in the main view
            match selected_menu_view {
                SelectedMenuView::Menu => {
                    let menu_paragraph =
                        monet::paint_menu(vec!["pricing", "exit"], menu_index as usize);
                    canvas.left_main_panel.menu_view = Some(canvas.left_main_panel.base_view);
                    f.render_widget(menu_paragraph, canvas.left_main_panel.menu_view.unwrap());
                }
                SelectedMenuView::Urn => {
                    let display_space_per_urn = 100 / config.urns.ilks.len() as u16;
                    let urn_display_constraints = config
                        .urns
                        .ilks
                        .iter()
                        .map(|_| Constraint::Percentage(display_space_per_urn))
                        .collect::<Vec<Constraint>>();
                    let urn_views = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(urn_display_constraints.as_ref())
                        .split(canvas.left_main_panel.base_view);
                    canvas.left_main_panel.set_urn_view(urn_views);
                    let urn_views = &canvas.left_main_panel.urn_view.unwrap();
                    for (i, urn) in data.urn_data.iter().enumerate() {
                        let urn_paragraph =
                            monet::paint_urn(urn, state.active_ilk.contains(&urn.ink_name));
                        f.render_widget(urn_paragraph, urn_views[i]);
                    }
                }
                SelectedMenuView::Pricing => {
                    let pricing_paragraph =
                        monet::paint_pricing_screen(data.mar, data.par, data.xau);
                    canvas.left_main_panel.menu_view = Some(canvas.left_main_panel.base_view);
                    f.render_widget(pricing_paragraph, canvas.left_main_panel.menu_view.unwrap());
                }
            }
            let (active_text, active_title) = match &state.selected_active_view {
                SelectedActiveView::Settings => (monet::paint_settings(&config), "settings"),
                SelectedActiveView::Ilk => {
                    if !state.active_ilk.is_empty() {
                        let all_ilk_data = &data.ilks;
                        // now, instead, iterate over all active ilks and append their data to the active view
                        let formatted_str = state
                            .active_ilk
                            .iter()
                            .enumerate()
                            .map(|(index, ilk)| {
                                format!(
                                    "ilk: {}\n{}",
                                    ilk,
                                    match all_ilk_data.get(index) {
                                        Some(ilk) => {
                                            monet::paint_ilk(ilk, data.last_refreshed)
                                        }
                                        _ => "  Awaiting ilk data...".to_string(),
                                    }
                                )
                            })
                            .collect::<Vec<String>>()
                            .join("\n");
                        (Paragraph::new(formatted_str), "ilk_view")
                    } else {
                        (Paragraph::new("No active view".to_string()), "active_view")
                    }
                }
                SelectedActiveView::NewPalm2 => {
                    if !data.logs.is_empty() {
                        let filtered_logs = data
                            .logs
                            .iter()
                            .filter(|log| {
                                !state.active_ilk.is_empty()
                                    && state.active_ilk.contains(&bytes32_to_string(log.ilk))
                                    || state.active_ilk.is_empty()
                            })
                            .collect::<Vec<&NewPalm2>>();

                        let logs_text = monet::paint_newpalm2s(filtered_logs, &canvas.color_map);
                        (logs_text, "frob/bail")
                    } else {
                        (Paragraph::new("Awaiting NewPalm2 event..."), "frob/bail")
                    }
                }
                _ => (Paragraph::new("No active view"), "active_view"),
            };

            let active_view_paragraph = active_text.block(
                Block::default()
                    .title(active_title)
                    .borders(Borders::ALL)
                    .border_style(style::Style::default().fg({
                        if state.selected_active_view != SelectedActiveView::Clear {
                            style::Color::LightYellow
                        } else {
                            style::Color::Gray
                        }
                    })),
            );
            f.render_widget(active_view_paragraph, canvas.right_main_pane.ilk_view);

            let footer_paragraph =
                monet::paint_footer(data.block, data.last_refreshed, ilk_help_message.as_str());
            f.render_widget(footer_paragraph, canvas.footer);
        })?;

        if rx.try_recv().is_ok() {
            // Data was refreshed
        } else if event::poll(std::time::Duration::from_millis(200))? {
            if let event::Event::Key(key) = event::read()? {
                match live_ilks_key_char.get(&key.code).cloned() {
                    Some(ilk) => {
                        let mut state = state.lock().unwrap();
                        match state.selected_active_view {
                            SelectedActiveView::Ilk => {
                                if !state.active_ilk.contains(&ilk) {
                                    state.active_ilk.push(ilk);
                                }
                            }
                            SelectedActiveView::NewPalm2 => {
                                if !state.active_ilk.contains(&ilk) {
                                    state.active_ilk.push(ilk);
                                }
                            }

                            SelectedActiveView::Clear => {
                                if !state.active_ilk.contains(&ilk) {
                                    state.active_ilk.push(ilk);
                                }
                                state.selected_active_view = SelectedActiveView::Ilk;
                            }
                            _ => {}
                        }
                    }
                    _ => match key.code {
                        KeyCode::Char('q') => {
                            break;
                        }
                        KeyCode::Char('c') => {
                            let mut state = state.lock().unwrap();
                            state.selected_active_view = SelectedActiveView::Clear;
                            state.active_ilk.clear();
                            state.active_new_palm_2 = None;
                            data.lock().unwrap().ilks.clear();
                            setting_active = false;
                        }
                        KeyCode::Char('s') => {
                            let mut state = state.lock().unwrap();
                            match state.selected_active_view {
                                SelectedActiveView::Settings => {
                                    state.selected_active_view = SelectedActiveView::Clear;
                                }

                                _ => {
                                    state.selected_active_view = SelectedActiveView::Settings;
                                }
                            };
                            setting_active = !setting_active;
                        }
                        KeyCode::Char('p') => {
                            let mut state = state.lock().unwrap();
                            state.active_ilk.pop();
                            data.lock().unwrap().ilks.pop();
                        }

                        KeyCode::Char('m') => {
                            match selected_menu_view {
                                SelectedMenuView::Urn => {
                                    selected_menu_view = SelectedMenuView::Menu;
                                    menu_index = 0;
                                }
                                SelectedMenuView::Menu => {
                                    selected_menu_view = SelectedMenuView::Urn;
                                    menu_index = -1;
                                }
                                SelectedMenuView::Pricing => {
                                    selected_menu_view = SelectedMenuView::Urn;
                                    menu_index = -1;
                                }
                            };
                        }
                        KeyCode::Char('\\') => {
                            match selected_market_view {
                                SelectedMarketView::MarAndPar => {
                                    selected_market_view = SelectedMarketView::DollarConversion;
                                }
                                SelectedMarketView::DollarConversion => {
                                    selected_market_view = SelectedMarketView::MarAndPar;
                                }
                            };
                        }

                        KeyCode::Char('f') => {
                            let mut state = state.lock().unwrap();
                            match state.selected_active_view {
                                SelectedActiveView::Clear => {
                                    state.selected_active_view = SelectedActiveView::NewPalm2;
                                    state.active_new_palm_2 = Some("art");
                                }
                                SelectedActiveView::NewPalm2 => {
                                    state.selected_active_view = SelectedActiveView::Clear;
                                    state.active_new_palm_2 = None;
                                }
                                _ => {}
                            }
                        }

                        // capture down arrow key
                        KeyCode::Down => {
                            if menu_index >= 0 {
                                menu_index += 1;
                            }
                        }
                        // capture up arrow key
                        KeyCode::Up => {
                            if menu_index >= 1 {
                                menu_index -= 1;
                            }
                        }
                        KeyCode::Enter => {
                            if let SelectedMenuView::Menu = selected_menu_view {
                                match menu_index {
                                    0 => {
                                        selected_menu_view = SelectedMenuView::Pricing;
                                    }
                                    1 => {
                                        break;
                                    }
                                    _ => {}
                                }
                            }
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

pub enum SelectedMenuView {
    Urn,
    Menu,
    Pricing,
}

pub enum SelectedMarketView {
    MarAndPar,
    DollarConversion,
}

#[derive(PartialEq, Eq)]
pub enum SelectedActiveView {
    Ilk,
    Settings,
    NewPalm2,
    Clear,
}
