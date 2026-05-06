//! `Effect` — what command handlers return. The client interprets each
//! variant in `App::apply_effect`. Effects represent semantic intent, not
//! wire-level commands; the client constructs `ClientCommand`s from app state.
