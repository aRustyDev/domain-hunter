mod web_driver;
mod util;

use crate::web_driver::expired_domains::*;
// use util::bad_words::*;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // let bad_words = get_bad_words(BadWordSource::File).unwrap();
    println!("{:?}", basically_selenium(CrawlTarget::ExpiredDomainsDotCom).await.unwrap());

}
