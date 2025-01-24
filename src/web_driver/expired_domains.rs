use crate::util::db::duck;
use crate::util::db::duck::Domain;
use crate::util::db::duck::DuckDbType;

use tracing::{Level, span, event, instrument};

use opentelemetry::{
  Context,
  KeyValue,
  trace::{
    Tracer,
    Span
  },
  global
};

use thirtyfour::{
  DesiredCapabilities,
  WebDriver,
  prelude::*,
  components::SelectElement,
};
use scraper::{Html, Selector, CaseSensitivity};
use itertools::max;
use tokio::time;
use dotenv::dotenv;

#[derive(Debug)]
pub enum CrawlTarget {
    ExpiredDomainsDotCom,
    Cloudflare,
}

struct FindBys {
    ids: Vec<String>,
    url: String,
}

// #[derive(Debug, Clone, Component)]
// pub struct CheckboxComponent {
//     base: WebElement,
//     #[by(tag = "label", first)]
//     label: ElementResolver<WebElement>,
//     #[by(css = "input[type='checkbox']")]
//     input: ElementResolver<WebElement>,
// }

// impl CheckBoxComponent {
//     pub async fn label_text(&self) -> WebDriverResult<String> {
//         let elem = self.label.resolve().await?;
//         elem.text().await
//     }

//     pub async fn is_ticked(&self) -> WebDriverResult<bool> {
//         let elem = self.input.resolve().await?;
//         let prop = elem.prop("checked").await?;
//         Ok(prop.unwrap_or_default() == "true")
//     }

//     pub async fn tick(&self) -> WebDriverResult<()> {
//         if !self.is_ticked().await? {
//             let elem = self.input.resolve().await?;
//             elem.click().await?;
//             assert!(self.is_ticked().await?);
//         }
//         Ok(())
//     }
// }

#[tracing::instrument]
pub async fn basically_selenium(target: CrawlTarget) -> WebDriverResult<Vec<String>> {
    dotenv().ok();
    let mut results = Vec::new();

    let site = match target {
      CrawlTarget::ExpiredDomainsDotCom => FindBys {
        ids: vec![
          "exp-collapse-link-TILE_NS11".to_string(), // Open "Dropped Domains where Pagerank > 0"
          "facet_0-0".to_string(), // Show only Available Domains
          "facet_2-0".to_string(), // Include '.com' domains
          "facet_2-1".to_string(), // Include '.net' domains
          "facet_2-2".to_string(), // Include '.org' domains
        ],
        url: "https://www.expired-domains.co/domains-available-by-range/last-31-days/".to_string(),
      },
      CrawlTarget::Cloudflare => todo!(),
    };
    
    event!(Level::INFO, "Creating WebDriver"); // vec![KeyValue::new("url", "http://localhost:4444")
    let browser = selenium("http://localhost:4444").await?;

    event!(Level::INFO, "Visiting Site"); // vec![KeyValue::new("url", site.url.clone())]
    browser.goto(site.url).await?;

    //  Sleep for 5 seconds
    time::sleep(time::Duration::from_millis(5000)).await;
    match target {
      CrawlTarget::ExpiredDomainsDotCom => {
        let mut span = span!(Level::INFO, "Clicking Page Elements").entered(); // vec![]
        // Setup the page
        for id in site.ids {
          let elem = browser.find(By::Id(id)).await?;
          elem.click().await?;
        }
        event!(Level::INFO, "Done Clicking Page Elements"); // vec![]
        // Sleep for 2 seconds
        time::sleep(time::Duration::from_millis(2000)).await;
        span.exit();
        
        span = span!(Level::INFO, "Finding Key Page Elements").entered(); // vec![]
        // Get the "Results per page" element
        let results_per_page_elem = browser.find(By::Id("tileTableTILE_NS11_length")).await?;
        let results_per_page_selector = SelectElement::new(&results_per_page_elem).await?;
        results_per_page_selector.select_by_value("100").await?; // Show 100 results per page
        // Sleep for 5 seconds
        event!(Level::INFO, "Done Finding Key Page Elements"); // vec![]
        time::sleep(time::Duration::from_millis(5000)).await;
        span.exit();


        span = span!(Level::INFO, "Getting page values (nth page, focus title, )").entered(); // vec![]
        // Get page values (nth page, focus title, )
        let pages_ul = Html::parse_fragment(&browser.find(By::Id("tileTableTILE_NS11_paginate")).await?.inner_html().await?);
        let last_page = get_last_page(&pages_ul.html())?;
        span.exit();


        span = span!(Level::INFO, "Verifying Title").entered(); // vec![]
        // Verify the title of the target box
        let title = browser.find(By::Id("exp-title-text-TILE_NS11")).await?.text().await?;
        if title != "Dropped Domains (PageRank > 0)" {
          panic!("Expected title 'Dropped Domains (PageRank > 0)'");
        }
        span.exit();

        // Cycle through pages
        // grab the content of the target table
        span = span!(Level::INFO, "Scraping Tables").entered(); // vec![]
        for i in 0..=last_page {
          event!(Level::INFO, "Scraping Table"); // vec![KeyValue::new("pg", i64::from(i))]
          let table = browser.find(By::Id("tileTableTILE_NS11_wrapper")).await?.find(By::Tag("table")).await?.outer_html().await?;
          get_records(&table, &mut results).await?;
          next_page(&browser).await?; // Click "Next" button
        }
        span.exit();
      }
      CrawlTarget::Cloudflare => {
        event!(Level::ERROR, "CrawlTarget Not Implemented yet"); // vec![KeyValue::new("foo", "1")]
        todo!()
      }
     }
    
     // Always explicitly close the browser.
     browser.quit().await?;
     event!(Level::INFO, "Closing the Browser"); // vec![]

      match dotenv::var("DB_TYPE").unwrap().as_str() {
        "sql" => {
          let mut span = span!(Level::INFO, "Writing Domains to DuckDB").entered(); // vec![]
          // span.add_event("Initializing DuckDB", vec![]);
          let mut conn = duck::db_init(DuckDbType::Persistent).unwrap();
          // span.add_event("Creating Transaction", vec![]);
          let tx = conn.transaction().unwrap();
          // span.add_event("Inserting Domains", vec![]);
          for domain in &results {
            duck::insert_domain(&tx, &Domain::new(domain, true, None)).unwrap();
          }
          // span.add_event("Done Inserting Domains", vec![]);
          tx.commit().unwrap();
          // span.add_event("Transaction Committed", vec![]);
          // span.add_event("Done Initializing DuckDB", vec![]);
          span.exit();
        },
        "graph" => {
          // span.add_event("Initializing IndraDB", vec![]);
          todo!()
        },
        &_ => todo!(),
      }

     // span.end();
     Ok(results)
}

#[tracing::instrument]
async fn next_page(driver: &WebDriver) -> WebDriverResult<()> {
    let next_button_elem = driver.find(By::Id("tileTableTILE_NS11_next")).await?.find(By::Tag("a")).await?;
    next_button_elem.click().await?;
    time::sleep(time::Duration::from_millis(100)).await;
    Ok(())
}

#[tracing::instrument]
async fn selenium(endpoint: &str) -> Result<WebDriver, WebDriverError> {
    let mut caps = DesiredCapabilities::chrome();
    caps.set_application_cache_enabled(false).unwrap();
    caps.set_headless().unwrap();
    caps.set_no_sandbox().unwrap();
    caps.set_disable_dev_shm_usage().unwrap();
    // --disable-extensions
    // start-maximized
    // enable-automation
    let browser = WebDriver::new(endpoint, caps).await?;
    browser.set_window_rect(0, 0, 1920, 1200).await?;
    browser.maximize_window().await?;
    Ok(browser)
}

#[tracing::instrument]
fn get_last_page(html: &String) -> WebDriverResult<u16> {
    event!(Level::INFO, "Finding Last Page Number"); // vec![]
    // Parse the HTML
    let fragment = Html::parse_fragment(html);

    // Create selectors (like regex)
    let ul_selector = Selector::parse("ul").unwrap();

    // Handle the List of Pages
    let ul = fragment.select(&ul_selector).next().unwrap();
    let pages = ul.descendent_elements().filter_map(|s| 
                match s.inner_html().parse::<u16>().is_ok() {
                  true => Some(s.inner_html().parse::<u16>().unwrap()),
                  false => None
                }
              ).collect::<Vec<u16>>();

    // Only return the maximum page number
    event!(Level::INFO, "Done Finding Last Page Number"); // vec![]
    Ok(max(pages).unwrap())
}

#[tracing::instrument]
async fn get_records(table_html: &String, results: &mut Vec<String>) -> WebDriverResult<()>{
    let mut records = Vec::new();

    // Parse the HTML
    let fragment = Html::parse_fragment(table_html);

    // Create selectors (like regex)
    let table_selector = Selector::parse("table").unwrap();
    
    // Get the table
    let table = fragment.select(&table_selector).next().unwrap();

    // Cycle each row; get the domain names
    for node in table.descendent_elements() { 
        // if the node has the class "exp-domain-link" -> add it to the records
        if node.value().has_class("exp-domain-link", CaseSensitivity::AsciiCaseInsensitive) {
          records.push(node.inner_html());
        }
    }

    results.append(&mut records);
    Ok(())
}
