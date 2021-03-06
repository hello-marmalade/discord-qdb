#[macro_use]
extern crate serenity;
#[macro_use(bson, doc)]
extern crate bson;
extern crate mongodb;
extern crate typemap;
extern crate yaml_rust;

mod yconfig;
mod commands;

use serenity::prelude::*;
use serenity::model::*;
use serenity::framework::standard::{Args, Command, DispatchError, StandardFramework, help_commands};
use serenity::utils;

use bson::Bson;
use mongodb::ThreadedClient;
use mongodb::db::ThreadedDatabase;

use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use typemap::Key;

struct CommandCounter;

impl Key for CommandCounter {
    type Value = HashMap<String, u64>;
}

struct Handler;

impl EventHandler for Handler {
    // Set a handler to be called on the 'on_ready' event. This is called when a shard is booted, and a READY payload is sent by Discord.
    // This payload contains data like the current user's guild Ids, current user data, private channels, and more.
    //
    // In this case, just print what the current user's username is.
    fn on_ready (&self, _: Context, ready: Ready) {
        println!("{} is connected.", ready.user.name);
    }
}

fn main() {

    // -- Read configuration file --
    let config = yconfig::read_config("./configuration.yaml");

    //  -- MongoDB Connector --
   let mdb_client = mongodb::Client::connect("localhost", 27017)
       .expect("Failed to initialize MongoDB standalone client.");
   let coll = mdb_client.db("test").collection("quotes");
   // let mdbdoc = doc! {
   //     "title": "Jaws",
   //     "array": [1, 2, 3],
   // };

   // coll.insert_one(mdbdoc.clone(), None)
   //     .ok().expect("Failed to insert document.");

   // let mut cursor = coll.find(Some(mdbdoc.clone()), None)
   //     .ok().expect("Failed to execute find.");

   // let item = cursor.next();

   // match item {
   //     Some(Ok(mdbdoc)) => match mdbdoc.get("title") {
   //         Some(&Bson::String(ref title)) => println!("{}", title),
   //         _ => panic!("Expected title to be a string!"),
   //     },
   //     Some(Err(_)) => panic!("Failed to get next from server!"),
   //     None => panic!("Server returned to results!"),
   // }

    // Create a new instance of the Client, logging in as a bot. This will automatically prepend your bot token with "Bot ", which is a requirement by discord for bot users.
    let mut client = serenity::Client::new(&config.token, Handler); {
        let mut data = client.data.lock();
        data.insert::<CommandCounter>(HashMap::default());
    }
    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform exponential backoff until it reconnects.
   // if let Err(why) = client.start() {
   //     println!("Client error: {:?}", why);
   // }

    client.with_framework(
        StandardFramework::new()
            .configure(|c| c
                       .allow_whitespace(true)
                       .on_mention(true)
                       .prefix(".")
                       .delimiters(vec![", ", ","])
                       )
            .before(|ctx, msg, command_name| {
                println!("Got command '{}' by user '{}'",
                         command_name,
                         msg.author.name);

                let mut data = ctx.data.lock();
                let counter = data.get_mut::<CommandCounter>().unwrap();
                let entry = counter.entry(command_name.to_string()).or_insert(0);
                *entry += 1;

                true
            })

            .after(|_, _, command_name, error| {
                match error {
                    Ok(()) => println!("Processed command '{}'", command_name),
                    Err(why) => println!("Command '{}' returned error {:?}", command_name, why),
                }
            })

            .on_dispatch_error(|_ctx, msg, error| {
                if let DispatchError::RateLimited(seconds) = error {
                    let _ = msg.channel_id.say(&format!("Try this again in {} seconds.", seconds));
                }
            })
            .bucket("complicated", 5, 30, 2)  // Can't be used more than 2x per 30s, with a 5s delay.
            .command("about", |c| c.exec_str("Quote Database Bot utilizing serenity, and MongoDB."))
            .command("help", |c| c.exec_help(help_commands::plain))
            .command("ping", |c| c.exec(commands::meta::ping))
            //.command("ccounter", |c| c
            //        .bucket("complicated")
            //        .exec(commands::meta::command_counter))
            .command("testbed", |c| c.exec(commands::testbed::testbed))
            //.command("addquote", |c| c.exec(addquote))
    );

    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}
