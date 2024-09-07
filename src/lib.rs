use playwright::{api::MouseButton, Playwright};
use serde::{Deserialize, Serialize};
use std::{error::Error, fs};

#[derive(Debug, Deserialize, Serialize)]
pub enum Action {
    // 既存のアクション
    GoTo {
        url: String,
    },
    Click {
        selector: String,
    },
    Input {
        selector: String,
        text: String,
    },
    Extract {
        selector: String,
        attribute: Option<String>,
    },
    Wait {
        milliseconds: u64,
    },
    // 新しいアクション
    Login {
        url: String,
        username_selector: String,
        password_selector: String,
        username: String,
        password: String,
        submit_selector: String,
    },
    Navigate {
        selector: String,
        attribute: String,
    },
    FillCheckbox {
        selector: String,
        checked: bool,
    },
    SelectDropdown {
        selector: String,
        option: String,
    },
    Hover {
        selector: String,
    },
    DoubleClick {
        selector: String,
    },
    RightClick {
        selector: String,
    },
    RunScript {
        script: String,
    },
    DownloadFile {
        url: String,
        dist_path: String,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ScrapingTask {
    pub name: String,
    pub actions: Vec<Action>,
}

pub async fn perform_action(
    page: &playwright::api::Page,
    action: &Action,
) -> Result<(), Box<dyn Error>> {
    // 各アクションに対応する処理を実行します
    match action {
        Action::GoTo { url } => page.goto_builder(url).goto().await?,
        Action::Click { selector } => {
            page.click_builder(selector).click().await?;
            return Ok(());
        }
        Action::Input { selector, text } => {
            page.fill_builder(selector, text).fill().await?;
            return Ok(());
        }
        Action::Extract {
            selector,
            attribute,
        } => {
            let elements = page.query_selector_all(selector).await?;
            for element in elements {
                let result = match attribute {
                    Some(attr) => element.get_attribute(attr).await?,
                    None => element.text_content().await?,
                };
                if let Some(content) = result {
                    println!("this content: {}", content); // ここでは単純に出力していますが、用途に応じて処理を変更可能
                    return Ok(());
                }
            }
            return Ok(());
        }
        Action::Wait { milliseconds } => {
            page.wait_for_timeout(*milliseconds as f64).await;
            return Ok(());
        }
        Action::Login {
            url,
            username_selector,
            password_selector,
            username,
            password,
            submit_selector,
        } => {
            page.goto_builder(url).goto().await?;
            page.fill_builder(username_selector, username)
                .fill()
                .await?;
            page.fill_builder(password_selector, password)
                .fill()
                .await?;
            page.click_builder(submit_selector).click().await?;
            page.wait_for_timeout(2000f64).await; // wait for login to complete
            return Ok(());
        }
        Action::Navigate {
            selector,
            attribute,
        } => {
            let element = match page.query_selector(selector).await? {
                Some(element) => element,
                None => return Ok(()),
            };
            let href = match element.get_attribute(&attribute).await {
                Ok(Some(href)) => href,
                _ => return Ok(()),
            };
            match page.goto_builder(&href).goto().await {
                Ok(_) => (),
                Err(e) => {
                    println!("Failed to navigate to {}: {:?}", href, e);
                    Err(e)?
                }
            }

            return Ok(());
        }
        Action::FillCheckbox { selector, checked } => {
            let checkbox = match page.query_selector(selector).await {
                Ok(checkbox) => checkbox.unwrap(),
                _ => return Err("Failed to find checkbox".into()),
            };

            let is_checked = checkbox.is_checked().await?;
            if is_checked != *checked {
                checkbox.click_builder().click().await?;
            }

            return Ok(());
        }
        Action::SelectDropdown { selector, option } => {
            page.select_option_builder(selector)
                .add_value(option.to_string());

            return Ok(());
        }
        Action::Hover { selector } => {
            page.hover_builder(selector).clear_force().goto().await?;
            return Ok(());
        }
        Action::DoubleClick { selector } => {
            page.dblclick_builder(selector).dblclick().await?;
            return Ok(());
        }
        Action::RightClick { selector } => {
            page.click_builder(selector)
                .button(MouseButton::Right)
                .click()
                .await?;
            return Ok(());
        }
        Action::RunScript { script } => {
            page.eval(script.as_str()).await?;
            return Ok(());
        }
        Action::DownloadFile { url, dist_path } => {
            let response = page.goto_builder(url).goto().await?;
            let body = response.unwrap().body().await?;
            std::fs::write(dist_path, body)?;
            return Ok(());
        }
    };

    Ok(())
}

pub async fn perform_scraping(task: &ScrapingTask) -> Result<Vec<String>, Box<dyn Error>> {
    let playwright = Playwright::initialize().await?;
    playwright.install_chromium()?;
    let browser = playwright.chromium().launcher().launch().await?;
    let context = browser.context_builder().build().await?;
    let page = context.new_page().await?;

    let mut results = Vec::new();

    for action in &task.actions {
        perform_action(&page, action).await?;
    }

    let run_before_unload = false;
    browser.close().await?;
    page.close(Some(run_before_unload)).await?;

    Ok(results)
}

pub fn load_json(filename: &str) -> Result<Vec<ScrapingTask>, Box<dyn Error>> {
    let contents = fs::read_to_string(filename)?;
    let tasks: Vec<ScrapingTask> = serde_json::from_str(&contents)?;
    Ok(tasks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_json() {
        let tasks: Vec<ScrapingTask> = load_json("tests/test.json").unwrap();
        println!("{:?}", tasks);
        assert_eq!(tasks.len(), 5);
        assert_eq!(tasks[0].name, "Task 1");
        assert_eq!(tasks[0].actions.len(), 2);
        assert_eq!(tasks[1].name, "Task 2");
        assert_eq!(tasks[1].actions.len(), 2);
    }

    #[tokio::test]
    async fn test_goto() {
        let playwright = Playwright::initialize().await.unwrap();
        playwright.install_chromium().unwrap();
        let browser = playwright
            .chromium()
            .launcher()
            .headless(false)
            .launch()
            .await
            .unwrap();
        let context = browser.context_builder().build().await.unwrap();
        let page = context.new_page().await.unwrap();

        let tasks = load_json("tests/test.json").unwrap();
        let actions = &tasks[4].actions;
        println!("{:?}", actions);
        for action in actions {
            perform_action(&page, action).await.unwrap();
        }

        browser.close().await.unwrap();
        page.close(None).await.unwrap();
    }
}
