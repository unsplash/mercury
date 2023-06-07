//! Helpers for linking to the Heroku web dashboard.

use url::Url;

/// The base URL of the Heroku web dashboard.
const DASHBOARD_BASE: &str = "https://dashboard.heroku.com";

/// Get a link to the activity page for a given app.
pub fn activity_page_url<T: ToString>(app_name: T) -> Url {
    let str = format!("{}/apps/{}/activity", DASHBOARD_BASE, app_name.to_string());

    // This unwrap is tested below.
    Url::parse(str.as_ref()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    quickcheck! {
      fn test_activity_page_url_never_panics(x: String) -> () {
          activity_page_url(x);
      }
    }
}
