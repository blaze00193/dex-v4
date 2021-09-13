use clap::{App, Arg};
use dex_cranker::Context;
use solana_clap_utils::{
    fee_payer::{fee_payer_arg, FEE_PAYER_ARG},
    input_parsers::{keypair_of, pubkey_of},
    input_validators::{self, is_pubkey},
};

fn main() {
    let matches = App::new("dex-crank")
        .version("0.1")
        .author("Bonfida")
        .about("Serum dex v3 cranking runtime")
        .arg(
            Arg::with_name("url")
                .short("u")
                .long("url")
                .help("A Solana RPC endpoint url")
                .takes_value(true),
        )
        .arg(fee_payer_arg())
        .arg(
            Arg::with_name("program_id")
                .short("p")
                .long("program-id")
                .help("The pubkey of the dex program")
                .takes_value(true)
                .validator(is_pubkey)
                .required(true),
        )
        .arg(
            Arg::with_name("market")
                .short("m")
                .long("market")
                .help("The pubkey of the dex market to interact with")
                .takes_value(true)
                .validator(is_pubkey)
                .required(true),
        )
        .arg(
            Arg::with_name("cranking-authority")
                .long("cranking-authority")
                .takes_value(true)
                .value_name("crank_authority")
                .validator(input_validators::is_valid_signer)
                .help("The key of the cranking authority holding at least a msrm"),
        )
        .arg(
            Arg::with_name("reward-target")
                .short("t")
                .long("reward-target")
                .help("The pubkey of the target account for SOL cranking rewards")
                .takes_value(true)
                .validator(is_pubkey)
                .required(true),
        )
        .get_matches();
    let endpoint = matches
        .value_of("url")
        .unwrap_or("https://solana-api.projectserum.com");
    let program_id = pubkey_of(&matches, "program_id").unwrap();
    let market = pubkey_of(&matches, "market").expect("Invalid market Pubkey");
    let reward_target = pubkey_of(&matches, "reward_target").expect("Invalid reward target pubkey");
    let fee_payer = keypair_of(&matches, FEE_PAYER_ARG.name).unwrap();
    let cranking_authority = keypair_of(&matches, "cranking-authority").unwrap();
    let context = Context {
        market,
        fee_payer,
        endpoint: String::from(endpoint),
        program_id,
        cranking_authority,
        reward_target,
    };
    context.crank();
}