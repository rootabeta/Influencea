use anyhow::Result;
use clap::Parser;
use quick_xml::de::from_str;
use serde::Deserialize;
use std::fs::File;
use std::io::Write;
use std::time::Duration;
use ureq::Agent;

#[derive(Parser, Debug)]
struct Args { 
    /// Your main nation, used to identify you to NS moderators
    #[arg(short, long)]
    nation: String, 

    /// Region to collect influence data for
    #[arg(short, long)]
    region: String,

    /// Census ID we want
    #[arg(short, long, default_value_t = 65)]
    census_id: u8
}

#[derive(Deserialize, Debug)]
struct Ranking { 
    #[serde(rename="NAME")]
    nation: String, 
    #[serde(rename="RANK")]
    rank: usize, 
    #[serde(rename="SCORE")]
    score: f64
}

#[derive(Deserialize, Debug)]
struct Nation { 
    #[serde(rename="NATION")]
    nation: Vec<Ranking>
}

#[derive(Deserialize, Debug)]
struct Region { 
    #[serde(rename="NATIONS")]
    nations: Nation
}

#[derive(Deserialize, Debug)]
struct APIResponse { 
    #[serde(rename="CENSUSRANKS")]
    region: Region
}


fn get_page(agent: &Agent, region: &str, census_id: &u8, start: &usize) -> Result<Vec<Ranking>> { 
    let url = format!("https://www.nationstates.net/cgi-bin/api.cgi?region={region}&q=censusranks;scale={census_id};start={start}");
    let response = agent.get(&url).call()?.into_string()?;
    let response: APIResponse = from_str(&response)?;
    Ok(response.region.nations.nation)
}

fn main() {
    let args = Args::parse();

    let mut prev_highest_ranking: usize;
    let mut highest_ranking: usize = 0;
    let mut rankings = Vec::new();

    let user_agent = format!(
        "Influencea/{0} (Developed by nation=Volstrostia; In use by nation={1})",
        env!("CARGO_PKG_VERSION"),
        args.nation,
    );

    let agent = ureq::AgentBuilder::new()
        .user_agent(&user_agent)
        .timeout(Duration::from_secs(5))
        .build();

    loop { 
        println!("Counting from rank {}", highest_ranking + 1);
        if let Ok(page_rankings) = get_page(&agent, &args.region, &args.census_id, &(highest_ranking + 1)) { 
            // Add the rankings
            for ranking in page_rankings { 
                rankings.push(ranking);
            }


            prev_highest_ranking = highest_ranking;
            highest_ranking = rankings.iter().max_by_key(|x| x.rank).unwrap().rank; // There will always
                                                                               // be at least one
            // No change to the highest ranking, meaning no new nations
            if prev_highest_ranking == highest_ranking { 
                break;
            }

            // Rate limit sleep
            std::thread::sleep(Duration::from_secs(2)); 
        } else { 
            break;
        }
    }

    rankings.sort_by(|x, y| x.score.partial_cmp(&y.score).unwrap());
    let filename = format!("rankings-{}-cid{}.csv", args.region, args.census_id);
    let mut file = File::create(filename).expect("Failed to create file");
    writeln!(file, "Region: {}, CensusID: {}", args.region, args.census_id).unwrap();
    writeln!(file, "Rank,Name,Value").unwrap();
    for rank in rankings { 
        let line = format!("{0},{1},{2}",
            rank.rank, 
            rank.nation, 
            rank.score
        );

        println!("{}", line);
        writeln!(file, "{}", line).unwrap();
    }
}
