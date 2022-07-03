
use crate::pbapi::{
    db_stat_response, get_request, iter_request, iter_response, key_request, mutation_request,
    update_request, DbStatResponse, GetOp, GetRequest, IterRequest, IterResponse, KeyRequest,
    MutationRequest, UpdateRequest,
};
use protobuf::{EnumOrUnknown, Message};

pub fn pbenc_key_req(key: &[u8]) -> Vec<u8> {
    let mut out_msg = KeyRequest::new();
    out_msg.magic = EnumOrUnknown::new(key_request::MagicNum::MAGIC);
    out_msg.key = key.to_vec();
    return out_msg.write_to_bytes().unwrap();
}

pub fn pbenc_get1_req(key: &[u8], skip_val: bool) -> Vec<u8> {
    let mut out_msg = GetRequest::new();
    out_msg.magic = EnumOrUnknown::new(get_request::MagicNum::MAGIC);

    let mut out_op = GetOp::new();
    out_op.key = key.to_vec();
    out_op.skip_val = skip_val;
    out_msg.ops.push(out_op);

    return out_msg.write_to_bytes().unwrap();
}

pub fn pbenc_mutate_ins1(key: &[u8], val: &[u8]) -> MutationRequest {
    let mut out_msg = MutationRequest::new();
    out_msg.magic = EnumOrUnknown::new(mutation_request::MagicNum::MAGIC);

    let mut out_upd = UpdateRequest::new();
    out_upd.magic = EnumOrUnknown::new(update_request::MagicNum::MAGIC);
    out_upd.key = key.to_vec();
    out_upd.value = val.to_vec();
    out_upd.is_insert = true;

    out_msg.reqs.push(out_upd);

    out_msg
}

pub fn pbenc_db_stat_resp(n_records: u64) -> Vec<u8> {
    let mut out_msg = DbStatResponse::new();
    out_msg.magic = EnumOrUnknown::new(db_stat_response::MagicNum::MAGIC);
    out_msg.n_records = n_records;

    return out_msg.write_to_bytes().unwrap();
}

pub fn pbenc_iter_resp(key_list: &crate::db::api::KeyList) -> Vec<u8> {
    let mut out_msg = IterResponse::new();
    out_msg.magic = EnumOrUnknown::new(iter_response::MagicNum::MAGIC);

    for key in &key_list.keys {
        out_msg.keys.push(key.clone());
    }
    out_msg.list_end = key_list.list_end;

    return out_msg.write_to_bytes().unwrap();
}

pub fn pbdec_iter_req(wiredata: &[u8]) -> Option<IterRequest> {
    match IterRequest::parse_from_bytes(wiredata) {
        Err(_e) => None,
        Ok(req) => {
            if req.magic != EnumOrUnknown::new(iter_request::MagicNum::MAGIC) {
                None
            } else {
                Some(req)
            }
        }
    }
}

pub fn pbdec_key_req(wiredata: &[u8]) -> Option<KeyRequest> {
    match KeyRequest::parse_from_bytes(wiredata) {
        Err(_e) => None,
        Ok(req) => {
            if req.magic != EnumOrUnknown::new(key_request::MagicNum::MAGIC) {
                None
            } else {
                Some(req)
            }
        }
    }
}

pub fn pbdec_update_req(wiredata: &[u8]) -> Option<UpdateRequest> {
    match UpdateRequest::parse_from_bytes(wiredata) {
        Err(_e) => None,
        Ok(req) => {
            if req.magic != EnumOrUnknown::new(update_request::MagicNum::MAGIC) {
                None
            } else {
                Some(req)
            }
        }
    }
}

pub fn pbdec_mget_req(wiredata: &[u8]) -> Option<GetRequest> {
    match GetRequest::parse_from_bytes(wiredata) {
        Err(_e) => None,
        Ok(req) => {
            if req.magic != EnumOrUnknown::new(get_request::MagicNum::MAGIC) {
                None
            } else {
                Some(req)
            }
        }
    }
}

pub fn pbdec_mutate_req(wiredata: &[u8]) -> Option<MutationRequest> {
    match MutationRequest::parse_from_bytes(wiredata) {
        Err(_e) => None,
        Ok(req) => {
            if req.magic != EnumOrUnknown::new(mutation_request::MagicNum::MAGIC) {
                None
            } else {
                Some(req)
            }
        }
    }
}
