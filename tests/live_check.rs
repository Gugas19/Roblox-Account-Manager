//! Every Roblox request behind the interface's pages, against the real Roblox, with the accounts of an app folder.
//! Read only: nothing is joined, bought, worn, sent, changed or removed. Ignored by the normal tests:
//!
//!     pwsh -File tools\live-check.ps1          with your accounts (it copies them, asks the password, cleans up)
//!     cargo test --test live_check public -- --ignored --nocapture      only what needs no account
//!
//! With accounts, always on a copy (RAM_LIVE_DIR): the app writes its defaults into the settings it opens. A password
//! protected file is opened with RAM_PASSWORD. Cookies and the password are never printed.

use ram::commands;
use std::future::Future;
use std::time::Instant;

struct Report {
    failed: usize,
}

impl Report {
    async fn check<T, F: Future<Output = Result<T, String>>>(&mut self, name: &str, work: F, describe: impl Fn(&T) -> Result<String, String>) {
        let started = Instant::now();
        let outcome = work.await.and_then(|v| describe(&v));
        let ms = started.elapsed().as_millis();
        match outcome {
            Ok(detail) => println!("  ok    {name} ({ms} ms): {detail}"),
            Err(why) => {
                self.failed += 1;
                println!("  FAIL  {name} ({ms} ms): {why}");
            }
        }
    }
}

/// What needs no account: game names, servers, search, universes, groups and pictures, a player's outfits
#[test]
#[ignore]
fn public() {
    let dir = std::env::temp_dir().join(format!("ram-live-public-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("ram-test-copy"), "").unwrap();
    let app = ram::app::init(dir.clone());
    let mut r = Report { failed: 0 };
    tauri::async_runtime::block_on(async {
        let api = &app.api;
        r.check("game name and icon", commands::joining::place_info("1818".into()).map(Ok), |v| v.as_ref().map(|v| format!("{}", v["name"])).ok_or_else(|| "nothing".to_string())).await;
        r.check("Games > Servers", async { api.servers(1818, "", None).await.map_err(|e| e.to_string()) }, |v| v["data"].as_array().map(|l| format!("{} servers", l.len())).ok_or_else(|| format!("no list: {}", v.to_string().chars().take(120).collect::<String>()))).await;
        r.check("Games > search", commands::games::search_games("obby".into(), 0), |l| if l.is_empty() { Err("nothing found".into()) } else { Ok(format!("{} games", l.len())) }).await;
        r.check("Games > the list with nothing typed", commands::games::search_games(String::new(), 0), |l| if l.is_empty() { Err("nothing listed".into()) } else { Ok(format!("{} games", l.len())) }).await;
        r.check("Games > Universe", async {
            let universe = commands::games::universe_id("1818".into()).await?;
            commands::games::universe_places(universe).await
        }, |l| if l.is_empty() { Err("no places".into()) } else { Ok(format!("{} places", l.len())) })
        .await;
        r.check("game name of a newer game", async { api.place_name(131346454575416).await.map_err(|e| e.to_string()) }, |n| n.clone().ok_or_else(|| "no name".to_string())).await;
        r.check("groups of builderman", async { api.groups_of(156).await.map_err(|e| e.to_string()) }, |l| if l.is_empty() { Err("none".into()) } else { Ok(format!("{} groups", l.len())) }).await;
        r.check("presence", async { api.presence(&[156, 1]).await.map_err(|e| e.to_string()) }, |l| if l.len() == 2 { Ok("2 answers".into()) } else { Err(format!("{} answers", l.len())) }).await;
        r.check("avatar headshots", async { Ok(api.headshots(&[156, 1]).await) }, |m| if m.len() == 2 { Ok("2 pictures".into()) } else { Err(format!("{} of 2", m.len())) }).await;
        r.check("item pictures", async { Ok(api.asset_thumbnails(&[1028606, 48474313]).await) }, |m| if m.len() == 2 { Ok("2 pictures".into()) } else { Err(format!("{} of 2", m.len())) }).await;
        r.check("follow check", commands::joining::follow_check("builderman".into()), |v| Ok(format!("in game: {}", v["inGame"]))).await;
        // without an account Roblox limits this one hard: a refusal must say so in words, never come back empty
        r.check("Avatar > Outfits of builderman (no account)", async {
            match commands::avatar::outfits("builderman".into()).await {
                Ok(list) if list.is_empty() => Err("came back empty".to_string()),
                Ok(list) => Ok(format!("{} outfits", list.len())),
                Err(why) if why.contains("limiting") => Ok(format!("refused with words: {why}")),
                Err(why) => Err(why),
            }
        }, |s| Ok(s.clone()))
        .await;
    });
    let _ = std::fs::remove_dir_all(&dir);
    println!("\n{} failed", r.failed);
    assert_eq!(r.failed, 0, "some Roblox requests failed");
}

#[test]
#[ignore]
fn accounts() {
    let Ok(dir) = std::env::var("RAM_LIVE_DIR") else { panic!("run tools\\live-check.ps1 (RAM_LIVE_DIR: a folder with a copy of the app's files)") };
    let app = ram::app::init(dir.into());
    let mut state = app.store.lock().unwrap().load();
    if matches!(state, ram::core::accounts::LoadState::NeedsPassword) {
        let password = std::env::var("RAM_PASSWORD").expect("the accounts are password protected: set RAM_PASSWORD");
        app.store.lock().unwrap().unlock(&password).expect("wrong password");
        state = ram::core::accounts::LoadState::Ready;
    }
    if !matches!(state, ram::core::accounts::LoadState::Ready) {
        eprintln!("The accounts didn't open: {state:?}");
        panic!("the accounts didn't open");
    }
    let accounts = app.store.lock().unwrap().accounts.clone();
    let Some(first) = accounts.first().cloned() else {
        panic!("no accounts");
    };
    let names: Vec<String> = accounts.iter().map(|a| a.username.clone()).collect();
    let mut r = Report { failed: 0 };
    println!("{} accounts, checking with {} (and all of them where a page takes several)\n", accounts.len(), first.username);

    tauri::async_runtime::block_on(async {
        let api = &app.api;
        let token = first.security_token.clone();

        // the session itself
        r.check("cookie works (csrf)", async { app.csrf(&first).await.map_err(|e| e.to_string()) }, |c| if c.is_empty() { Err("empty".into()) } else { Ok("got a token".into()) }).await;
        r.check("username lookup", async { api.user_id(&first.username, Some(&token)).await.map_err(|e| e.to_string()) }, |id| if *id == first.user_id { Ok(format!("{id}")) } else { Err(format!("{id} isn't {}", first.user_id)) }).await;

        // Avatar
        r.check("Avatar > Outfits", commands::avatar::outfits(first.username.clone()), |list| Ok(format!("{} outfits", list.len()))).await;
        r.check("Avatar > Outfits of builderman", commands::avatar::outfits("builderman".into()), |list| if list.is_empty() { Err("none, builderman has outfits".into()) } else { Ok(format!("{} outfits", list.len())) }).await;
        r.check("Avatar > Inventory", async { Ok(commands::items::inventories(vec![first.username.clone()]).await) }, |list| {
            let v = serde_json::to_value(list).unwrap();
            match v[0]["error"].as_str() {
                Some(e) => Err(e.to_string()),
                None => Ok(format!("{} items", v[0]["list"].as_array().map(|l| l.len()).unwrap_or(0))),
            }
        })
        .await;
        r.check("item pictures", async { Ok(api.asset_thumbnails(&[1028606, 48474313]).await) }, |m| if m.len() == 2 { Ok("2 pictures".into()) } else { Err(format!("{} of 2", m.len())) }).await;

        // Friends
        r.check("Friends > list", commands::friends::friends_list(first.username.clone()), |list| {
            let v = serde_json::to_value(list).unwrap();
            let all = v.as_array().map(|l| l.len()).unwrap_or(0);
            let named = v.as_array().map(|l| l.iter().filter(|f| !f["name"].as_str().unwrap_or("").is_empty()).count()).unwrap_or(0);
            let pictures = v.as_array().map(|l| l.iter().filter(|f| f["avatar"].is_string()).count()).unwrap_or(0);
            if named < all { Err(format!("{all} friends, {} without a name", all - named)) } else { Ok(format!("{all} friends, {pictures} with a picture")) }
        })
        .await;
        r.check("Friends > requests", commands::friends::friend_requests(first.username.clone()), |list| Ok(format!("{} requests", list.len()))).await;
        r.check("Friends > visibility", commands::friends::visibility_get(first.username.clone()), |list| {
            if list.len() != 2 {
                return Err(format!("{} settings, expected 2", list.len()));
            }
            Ok(list.iter().map(|v| format!("{} = {} ({} choices)", v.field, v.current, v.options.len())).collect::<Vec<_>>().join("; "))
        })
        .await;

        // Groups
        r.check("Groups", async { Ok(commands::items::groups(names.clone()).await) }, |g| {
            let v = serde_json::to_value(g).unwrap();
            let errors: Vec<String> = v["accounts"].as_array().into_iter().flatten().filter_map(|a| a["error"].as_str().map(|e| format!("{}: {e}", a["account"]))).collect();
            if !errors.is_empty() {
                return Err(errors.join("; "));
            }
            let count: usize = v["accounts"].as_array().into_iter().flatten().map(|a| a["list"].as_array().map(|l| l.len()).unwrap_or(0)).sum();
            Ok(format!("{count} memberships, {} icons", v["icons"].as_object().map(|o| o.len()).unwrap_or(0)))
        })
        .await;

        // presence (Status column, History)
        let ids: Vec<i64> = accounts.iter().map(|a| a.user_id).collect();
        r.check("presence of the accounts", async { api.presence(&ids).await.map_err(|e| e.to_string()) }, |list| if list.len() == ids.len() { Ok(format!("{} answers", list.len())) } else { Err(format!("{} answers for {} accounts", list.len(), ids.len())) }).await;
        r.check("own presence (game name)", async {
            let csrf = app.csrf(&first).await.map_err(|e| e.to_string())?;
            api.own_presence(first.user_id, &token, &csrf).await.map_err(|e| e.to_string())
        }, |p| p.as_ref().map(|p| format!("kind {}, location {:?}", p.kind, p.last_location)).ok_or_else(|| "no answer".to_string()))
        .await;

        // Games
        r.check("game name and icon", commands::joining::place_info("1818".into()).map(Ok), |v| v.as_ref().map(|v| format!("{}", v["name"])).ok_or_else(|| "nothing".to_string())).await;
        r.check("Games > Servers", async { api.servers(1818, "", None).await.map_err(|e| e.to_string()) }, |v| v["data"].as_array().map(|l| format!("{} servers", l.len())).ok_or_else(|| format!("no list: {}", v.to_string().chars().take(120).collect::<String>()))).await;
        r.check("Games > search", commands::games::search_games("obby".into(), 0), |l| if l.is_empty() { Err("nothing found".into()) } else { Ok(format!("{} games", l.len())) }).await;
        r.check("Games > Universe", async {
            let universe = commands::games::universe_id("1818".into()).await?;
            commands::games::universe_places(universe).await
        }, |l| if l.is_empty() { Err("no places".into()) } else { Ok(format!("{} places", l.len())) })
        .await;
        r.check("follow check (a player in a game?)", commands::joining::follow_check("builderman".into()), |v| Ok(format!("in game: {}", v["inGame"]))).await;
        r.check("avatar headshots", async { Ok(api.headshots(&ids).await) }, |m| if m.len() == ids.len() { Ok(format!("{} pictures", m.len())) } else { Err(format!("{} of {}", m.len(), ids.len())) }).await;
    });

    println!("\n{} failed", r.failed);
    assert_eq!(r.failed, 0, "some Roblox requests failed");
}

trait MapOk: Future + Sized {
    fn map<T, F: FnOnce(Self::Output) -> T>(self, f: F) -> impl Future<Output = T> {
        async move { f(self.await) }
    }
}
impl<F: Future> MapOk for F {}
