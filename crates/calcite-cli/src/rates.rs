use std::{
    env, fs, io,
    path::{Path, PathBuf},
    time::Duration,
};

use calcite_core::{Currency, ExchangeRateSnapshot};
use calcite_rates::{
    EcbProvider, ExchangeRateProvider, FallbackProvider, FrankfurterProvider, ProviderError,
    RateRequest, load_json_snapshot, save_json_snapshot,
};

const CACHE_MAX_AGE: Duration = Duration::from_secs(12 * 60 * 60);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderChoice {
    Fallback,
    Frankfurter,
    Ecb,
}

pub struct CachedRates {
    pub snapshot: ExchangeRateSnapshot,
    pub fresh: bool,
}

pub fn cache_path() -> PathBuf {
    if let Some(path) = env::var_os("CALCITE_RATE_CACHE").filter(|path| !path.is_empty()) {
        return PathBuf::from(path);
    }
    if let Some(directory) = env::var_os("XDG_CACHE_HOME").filter(|path| !path.is_empty()) {
        return PathBuf::from(directory).join("calcite/rates.json");
    }
    if let Some(home) = env::var_os("HOME").filter(|path| !path.is_empty()) {
        return PathBuf::from(home).join(".cache/calcite/rates.json");
    }

    // Trust me, this might look janky, but it works, so I don't dare touch it.
    env::temp_dir().join("calcite/rates.json")
}

pub fn load_cache(path: &Path) -> io::Result<Option<CachedRates>> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let snapshot = load_json_snapshot(&source)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let fresh = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .and_then(|modified| modified.elapsed().map_err(io::Error::other))
        .is_ok_and(|age| age <= CACHE_MAX_AGE);
    Ok(Some(CachedRates { snapshot, fresh }))
}

pub async fn fetch_rates(provider: ProviderChoice) -> Result<ExchangeRateSnapshot, ProviderError> {
    let request = RateRequest::new(Currency::Usd, []);
    match provider {
        ProviderChoice::Frankfurter => FrankfurterProvider::default().fetch(&request).await,
        ProviderChoice::Ecb => EcbProvider::default().fetch(&request).await,
        ProviderChoice::Fallback => {
            let mut providers = FallbackProvider::default();
            providers.push(FrankfurterProvider::default());
            providers.push(EcbProvider::default());
            providers.fetch(&request).await
        }
    }
}

pub fn save_cache(path: &Path, snapshot: &ExchangeRateSnapshot) -> io::Result<()> {
    let source = save_json_snapshot(snapshot).map_err(io::Error::other)?;
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "rate cache has no parent directory",
        )
    })?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".rates-{}.tmp", std::process::id()));
    fs::write(&temporary, source)?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}
