use thirtyfour::{DesiredCapabilities, WebDriver};
use thirtyfour::prelude::*;
use thirtyfour::components::SelectElement;
use thirtyfour::error::WebDriverErrorInfo;
use scraper::{Html, Selector};
use scraper::CaseSensitivity;
use itertools::max;
use tokio::time;

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

pub async fn basically_selenium(target: CrawlTarget) -> WebDriverResult<Vec<String>> {
     let mut results = Vec::new();
     let mut caps = DesiredCapabilities::chrome();

     caps.set_application_cache_enabled(false).unwrap();
     caps.set_headless().unwrap();
     caps.set_no_sandbox().unwrap();
     caps.set_disable_dev_shm_usage().unwrap();
     // --disable-extensions
     // start-maximized
     // enable-automation
     let browser = WebDriver::new("http://localhost:4444", caps).await?;
     browser.set_window_rect(0, 0, 1920, 1200).await?;
     browser.maximize_window().await?;

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

     browser.goto(site.url).await?;
    //  Sleep for 5 seconds
     time::sleep(time::Duration::from_millis(5000)).await;
     match target {
        CrawlTarget::ExpiredDomainsDotCom => {
          // Setup the page
          for id in site.ids {
            let elem = browser.find(By::Id(id)).await?;
            elem.click().await?;
          }
          // Sleep for 2 seconds
          time::sleep(time::Duration::from_millis(2000)).await;

          // Get the "Results per page" element
          let results_per_page_elem = browser.find(By::Id("tileTableTILE_NS11_length")).await?;
          let results_per_page_selector = SelectElement::new(&results_per_page_elem).await?;
          results_per_page_selector.select_by_value("100").await?; // Show 100 results per page
          // Sleep for 5 seconds
          time::sleep(time::Duration::from_millis(5000)).await;

          // Get page values (nth page, focus title, )
          let pages_ul = Html::parse_fragment(&browser.find(By::Id("tileTableTILE_NS11_paginate")).await?.inner_html().await?);
          let last_page = get_last_page(&pages_ul.html())?;

          // Verify the title of the target box
          let title = browser.find(By::Id("exp-title-text-TILE_NS11")).await?.text().await?;
          if title != "Dropped Domains (PageRank > 0)" {
            panic!("Expected title 'Dropped Domains (PageRank > 0)'");
          }
          // Cycle through pages
          // grab the content of the target table
          for _ in 0..=last_page {
            let table = browser.find(By::Id("tileTableTILE_NS11_wrapper")).await?.find(By::Tag("table")).await?.outer_html().await?;
            get_records(&table, &mut results).await?;
            next_page(&browser).await?; // Click "Next" button
          }

        }
        CrawlTarget::Cloudflare => {
            todo!()
        }
     }
    
     // Always explicitly close the browser.
     browser.quit().await?;

     Ok(results)
}

async fn next_page(driver: &WebDriver) -> WebDriverResult<()> {
    let next_button_elem = driver.find(By::Id("tileTableTILE_NS11_next")).await?.find(By::Tag("a")).await?;
    next_button_elem.click().await?;
    time::sleep(time::Duration::from_millis(100)).await;
    Ok(())
}

fn get_last_page(html: &String) -> WebDriverResult<u16> {
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
    Ok(max(pages).unwrap())
}

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
