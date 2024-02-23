use std::time::{Duration, Instant};
use regex::Regex;
use reqwest::{Client, Url};
use reqwest::header::CONTENT_TYPE;
use tui_textarea::TextArea;
use crate::app::app::App;
use crate::request::auth::{next_auth};
use crate::request::auth::Auth::*;
use crate::request::body::{ContentType, next_content_type};
use crate::request::method::next_method;
use crate::utils::stateful_custom_table::Param;

impl App<'_> {
    /* URL */
    pub fn modify_request_url(&mut self) {
        let input_text = self.url_text_input.text.clone();
        let selected_request_index = &self.collections_tree.selected.unwrap();
        let selected_request = &mut self.collections[selected_request_index.0].requests[selected_request_index.1];

        let url_parts = input_text.split_once("?");

        let final_url: String;
        let query_params: &str;

        if let Some((url, found_query_params)) = url_parts {
            final_url = url.to_string();
            query_params = found_query_params;
        }
        else {
            final_url = input_text;
            query_params = "";
        }


        let mut new_params_to_add: Vec<Param> = vec![];
        let mut existing_params_found_indexes: Vec<usize> = vec![];

        let query_params_pattern = Regex::new(r"(&?([^=]+)=([^&]+))").unwrap();

        for (_, [_, param_name, value]) in query_params_pattern.captures_iter(query_params).map(|c| c.extract()) {
            let mut url_param_found = false;

            for (index, existing_param) in selected_request.params.iter_mut().enumerate() {
                if param_name == existing_param.data.0 && existing_param.enabled {
                    existing_param.data.1 = value.to_string();
                    url_param_found = true;
                    existing_params_found_indexes.push(index);
                }
            }

            if !url_param_found {
                let new_param = Param {
                    enabled: true,
                    data: (param_name.to_string(), value.to_string()),
                };

                new_params_to_add.push(new_param);
            }
        }

        let param_indexes = selected_request.params.len();

        for param_index in 0..param_indexes {
            if !existing_params_found_indexes.contains(&param_index) {
                selected_request.params.remove(param_index);
            }
        }

        for new_param in new_params_to_add {
            selected_request.params.push(new_param);
        }

        selected_request.url = final_url;

        // In case new params were inputted or deleted
        self.update_params_selection();

        self.save_collections_to_file();
        self.select_request_state();
    }

    /* METHOD */

    pub fn modify_request_method(&mut self) {
        let selected_request_index = &self.collections_tree.selected.unwrap();
        let selected_request = &mut self.collections[selected_request_index.0].requests[selected_request_index.1];

        let next_method = next_method(&selected_request.method);

        selected_request.method = next_method;

        self.save_collections_to_file();
    }

    /* PARAMS */
    /// Reset selection of if params are provided, either set it to none
    pub fn update_params_selection(&mut self) {
        let selected_request_index = &self.collections_tree.selected.unwrap();
        let selected_request = &self.collections[selected_request_index.0].requests[selected_request_index.1];

        match !selected_request.params.is_empty() {
            true => {
                self.request_param_table.selection = Some((0, 0));
                self.request_param_table.left_state.select(Some(0));
                self.request_param_table.right_state.select(Some(0));
            },
            false => {
                self.request_param_table.selection = None;
                self.request_param_table.left_state.select(None);
                self.request_param_table.right_state.select(None);
            }
        }
    }

    pub fn toggle_params_table_row(&mut self) {
        if self.request_param_table.rows.is_empty() {
            return;
        }

        let selected_request_index = &self.collections_tree.selected.unwrap();
        let selected_request = &mut self.collections[selected_request_index.0].requests[selected_request_index.1];

        let row = self.request_param_table.selection.unwrap().0;

        selected_request.params[row].enabled = !selected_request.params[row].enabled;

        self.save_collections_to_file();
        self.update_inputs();
    }

    pub fn modify_request_param(&mut self) {
        let selected_request_index = &self.collections_tree.selected.unwrap();
        let selected_request = &mut self.collections[selected_request_index.0].requests[selected_request_index.1];

        let selection = self.request_param_table.selection.unwrap();
        let input_text = &self.request_param_table.param_selection_text_input.text;

        match selection {
            (_, 0) => selected_request.params[selection.0].data.0 = input_text.clone(),
            (_, 1) => selected_request.params[selection.0].data.1 = input_text.clone(),
            (_, _) => {}
        };

        self.save_collections_to_file();
        self.select_request_state();
    }

    /* AUTH */

    pub fn modify_request_auth(&mut self) {
        let selected_request_index = &self.collections_tree.selected.unwrap();
        let selected_request = &mut self.collections[selected_request_index.0].requests[selected_request_index.1];

        selected_request.auth = next_auth(&selected_request.auth);

        self.save_collections_to_file();
        self.load_request_auth_param_tab();
    }

    pub fn select_request_auth_input_text(&mut self) {
        let selected_request_index = &self.collections_tree.selected.unwrap();
        let selected_request = &mut self.collections[selected_request_index.0].requests[selected_request_index.1];

        match selected_request.auth {
            NoAuth => {}
            BasicAuth(_, _) => match self.auth_text_input_selection.selected {
                0 => self.edit_request_auth_username_state(),
                1 => self.edit_request_auth_password_state(),
                _ => {}
            },
            BearerToken(_) => match self.auth_text_input_selection.selected {
                0 => self.edit_request_auth_bearer_token_state(),
                _ => {}
            }
        }
    }

    pub fn modify_request_auth_basic_username(&mut self) {
        let input_text = self.auth_basic_username_text_input.text.clone();

        let selected_request_index = &self.collections_tree.selected.unwrap();
        let selected_request = &mut self.collections[selected_request_index.0].requests[selected_request_index.1];

        match &selected_request.auth {
            BasicAuth(_, password) => {
                selected_request.auth = BasicAuth(input_text, password.to_string());
            }
            _ => {}
        }

        self.save_collections_to_file();
        self.select_request_state();
    }

    pub fn modify_request_auth_basic_password(&mut self) {
        let input_text = self.auth_basic_password_text_input.text.clone();

        let selected_request_index = &self.collections_tree.selected.unwrap();
        let selected_request = &mut self.collections[selected_request_index.0].requests[selected_request_index.1];

        match &selected_request.auth {
            BasicAuth(username, _) => {
                selected_request.auth = BasicAuth(username.to_string(), input_text);
            }
            _ => {}
        }

        self.save_collections_to_file();
        self.select_request_state();
    }

    pub fn modify_request_auth_bearer_token(&mut self) {
        let input_text = self.auth_bearer_token_text_input.text.clone();

        let selected_request_index = &self.collections_tree.selected.unwrap();
        let selected_request = &mut self.collections[selected_request_index.0].requests[selected_request_index.1];

        match &selected_request.auth {
            BearerToken(_) => {
                selected_request.auth = BearerToken(input_text);
            }
            _ => {}
        }

        self.save_collections_to_file();
        self.select_request_state();
    }

    /* BODY */

    pub fn refresh_body_textarea(&mut self, text: String) {
        let lines: Vec<String> = text
            .lines()
            .map(|line| line.to_string())
            .collect();

        self.body_text_area = TextArea::new(lines);
    }

    pub fn modify_request_body(&mut self) {
        let selected_request_index = &self.collections_tree.selected.unwrap();
        let selected_request = &mut self.collections[selected_request_index.0].requests[selected_request_index.1];

        let body = self.body_text_area.lines().join("\n");

        let new_body = match selected_request.body {
            ContentType::NoBody => ContentType::NoBody,
            ContentType::Raw(_) => ContentType::Raw(body.clone()),
            ContentType::Json(_) => ContentType::Json(body.clone()),
            ContentType::Xml(_) => ContentType::Xml(body.clone()),
            ContentType::Html(_) => ContentType::Html(body.clone())
        };

        selected_request.body = new_body;

        self.save_collections_to_file();
        self.select_request_state();
    }

    pub fn modify_request_content_type(&mut self) {
        let selected_request_index = &self.collections_tree.selected.unwrap();
        let selected_request = &mut self.collections[selected_request_index.0].requests[selected_request_index.1];

        selected_request.body = next_content_type(&selected_request.body);

        self.load_request_body_param_tab();
    }

    pub fn quit_request_body(&mut self) {
        self.update_inputs();
        self.select_request_state();
    }

    /* REQUEST */

    pub async fn send_request(&mut self) {
        let selected_request_index = &self.collections_tree.selected.unwrap();
        let selected_request = &mut self.collections[selected_request_index.0].requests[selected_request_index.1];

        let params: Vec<(String, String)> = selected_request.params
            .iter()
            .filter_map(|param| {
                if param.enabled {
                    Some(param.data.clone())
                }
                else {
                    None
                }
            })
            .collect();

        let client = Client::new();

        let url = Url::parse_with_params(&selected_request.url, params).unwrap();

        let mut request = client.request(
            selected_request.method.as_reqwest(),
            url
        );

        match &selected_request.auth {
            NoAuth => {}
            BasicAuth(username, password) => {
                request = request.basic_auth(username, Some(password));
            }
            BearerToken(bearer_token) => {
                request = request.bearer_auth(bearer_token);
            }
        }

        match &selected_request.body {
            ContentType::NoBody => {},
            ContentType::Raw(body) | ContentType::Json(body) | ContentType::Xml(body) | ContentType::Html(body) => {
                request = request
                    .header(CONTENT_TYPE, selected_request.body.to_content_type())
                    .body(body.to_string());
            }
        };

        let request_start = Instant::now();
        let elapsed_time: Duration;

        match request.send().await {
            Ok(response) => {
                let status_code = response.status().to_string();

                let headers = response.headers().clone()
                    .iter()
                    .map(|(header_name, header_value)| {
                        format!("{}: {:?}", header_name.to_string(), header_value)
                    })
                    .collect::<Vec<String>>()
                    .join("\n");

                let cookies = response.cookies()
                    .map(|cookie| {
                        format!("{}: {}", cookie.name(), cookie.value())
                    })
                    .collect::<Vec<String>>()
                    .join("\n");

                let result_body = response.text().await.unwrap();

                selected_request.result.status_code = Some(status_code);
                selected_request.result.body = Some(result_body);
                selected_request.result.cookies = Some(cookies);
                selected_request.result.headers = Some(headers);
            },
            Err(error) => {
                let response_status_code;

                if let Some(status_code) = error.status() {
                    response_status_code = Some(status_code.to_string());
                }
                else {
                    response_status_code = None;
                }
                let result_body = error.to_string();


                selected_request.result.status_code = response_status_code;
                selected_request.result.body = Some(result_body);
                selected_request.result.cookies = None;
                selected_request.result.headers = None;
            }
        };

        elapsed_time = request_start.elapsed();

        selected_request.result.duration = Some(format!("{:?}", elapsed_time));

        self.refresh_result_scrollbar();
    }
}