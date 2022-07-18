extern crate reqwest;
use crate::codec;

pub const API_BASEURI: &'static str = "/api";

use crate::pbapi::{get_response, DbStatResponse, GetResponse, MutationRequest};
use protobuf::{EnumOrUnknown, Message};
use reqwest::StatusCode;

pub struct KvdbClient {
    client: reqwest::Client,
    pub db_id: String,
    pub endpoint: String,
}

impl KvdbClient {
    pub fn new(endpoint_: String, db_id_: String) -> KvdbClient {
        KvdbClient {
            client: reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap(),
            db_id: db_id_,
            endpoint: endpoint_,
        }
    }

    pub async fn get1(&mut self, key: String) -> Option<Vec<u8>> {
        let basepath = format!("{}{}/{}/", self.endpoint, API_BASEURI, self.db_id);
        let get_url = format!("{}mget", basepath);

        // encode get request
        let out_bytes = codec::pbenc_get1_req(key.as_bytes(), false);

        // exec get request; key1 should exist and match value, following batch
        let resp_res = self
            .client
            .post(&get_url)
            .body(out_bytes.clone())
            .send()
            .await;
        match resp_res {
            Ok(resp) => {
                if resp.status() != StatusCode::OK {
                    return None;
                }

                match resp.bytes().await {
                    Ok(bytes) => match GetResponse::parse_from_bytes(&bytes) {
                        Err(_e) => None,
                        Ok(in_resp) => {
                            if in_resp.magic != EnumOrUnknown::new(get_response::MagicNum::MAGIC)
                                || in_resp.res.len() != 1
                                || !in_resp.res[0].is_ok
                            {
                                None
                            } else {
                                Some(in_resp.res[0].val.clone())
                            }
                        }
                    },
                    Err(_e) => None,
                }
            }
            Err(_e) => None,
        }
    }

    pub async fn mutate(&mut self, mut_req: &MutationRequest) -> bool {
        let basepath = format!("{}{}/{}/", self.endpoint, API_BASEURI, self.db_id);
        let mutate_url = format!("{}mutate", basepath);

        // encode mutation request
        let out_bytes = mut_req.write_to_bytes().unwrap();

        // exec mutation request
        let resp_res = self.client.post(&mutate_url).body(out_bytes).send().await;
        match resp_res {
            Ok(resp) => {
                if resp.status() == StatusCode::OK {
                    match resp.text().await {
                        Ok(_body) => true,
                        Err(_e) => false,
                    }
                } else {
                    false
                }
            }
            Err(_e) => false,
        }
    }

    pub async fn put1(&mut self, key: String, value: String) -> bool {
        // encode put request
        let out_req = codec::pbenc_mutate_ins1(key.as_bytes(), value.as_bytes());

        self.mutate(&out_req).await
    }

    pub async fn del1(&mut self, key: String) -> bool {
        let basepath = format!("{}{}/{}/", self.endpoint, API_BASEURI, self.db_id);
        let del_url = format!("{}del", basepath);

        // encode del request
        let out_bytes = codec::pbenc_key_req(key.as_bytes());

        // exec del request
        let resp_res = self.client.post(&del_url).body(out_bytes).send().await;
        match resp_res {
            Ok(resp) => {
                if resp.status() == StatusCode::OK {
                    match resp.text().await {
                        Ok(_body) => true,
                        Err(_e) => false,
                    }
                } else {
                    false
                }
            }
            Err(_e) => false,
        }
    }

    pub async fn stat(&mut self) -> Option<DbStatResponse> {
        let basepath = format!("{}{}/{}/", self.endpoint, API_BASEURI, self.db_id);
        let stat_url = format!("{}stat", basepath);

        // exec db-stat request
        let resp_res = self.client.get(&stat_url).send().await;
        if resp_res.is_err() {
            return None;
        }
        let resp = resp_res.unwrap();
        if resp.status() != StatusCode::OK {
            return None;
        }

        // decode protobuf list-of-keys response
        match resp.bytes().await {
            Ok(bytes) => match DbStatResponse::parse_from_bytes(&bytes) {
                Err(_e) => None,
                Ok(req) => Some(req),
            },
            Err(_e) => None,
        }
    }

    pub async fn serverinfo(&mut self) -> Option<Vec<u8>> {
        let serverinfo_url = format!("{}/", self.endpoint);

        // exec db-stat request
        let resp_res = self.client.get(&serverinfo_url).send().await;
        match resp_res {
            Err(_e) => None,
            Ok(resp) => {
                if resp.status() == StatusCode::OK {
                    // receive JSON response
                    match resp.bytes().await {
                        Ok(json_bytes) => Some(json_bytes.to_vec()),
                        Err(_e) => None,
                    }
                } else {
                    None
                }
            }
        }
    }
}
