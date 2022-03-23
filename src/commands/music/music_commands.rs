use log::error;
use serenity::client::Context;
use serenity::framework::standard::{CommandResult, Args};
use serenity::model::channel::Message;
use serenity::framework::standard::macros::{command};
use serenity::prelude::{Mentionable};
use crate::Lavalink;

#[command]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "`help, join, leave, play, now_playing, skip`").await?;

    Ok(())
}

async fn join_interactive(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            msg.reply(&ctx.http, "Join a voice channel first.").await?;
            return Ok(());
        }
    };

    let manager = songbird::get(ctx).await.unwrap().clone();

    let (_, handler) = manager.join_gateway(guild_id, connect_to).await;

    match handler {
        Ok(connection_info) => {
            let data = ctx.data.read().await;
            let lava_client = data.get::<Lavalink>().unwrap().clone();
            lava_client.create_session(&connection_info).await?;

            msg.channel_id
                .say(&ctx.http, &format!("Joined {}", connect_to.mention()))
                .await?;
        }
        Err(why) => {
            msg.channel_id
                .say(&ctx.http, format!("Error joining the channel: {}", why))
                .await?;
        }
    }

    Ok(())
}

#[command]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    join_interactive(ctx, msg).await
}

#[command]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx).await.unwrap().clone();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild_id).await {
            msg.channel_id
                .say(&ctx.http, format!("Failed: {:?}", e))
                .await?;
        }

        {
            let data = ctx.data.read().await;
            let lava_client = data.get::<Lavalink>().unwrap().clone();
            lava_client.destroy(guild_id).await?;
        }

        msg.channel_id.say(&ctx.http, "Left voice channel").await?;
    } else {
        msg.reply(&ctx.http, "Not in a voice channel").await?;
    }

    Ok(())
}

#[command]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {

    if args.is_empty() {
        msg.channel_id.say(&ctx.http,"Please enter a query or link.").await?;
    }

    let query = args.message().to_string();

    let guild_id = match ctx.cache.guild_channel(msg.channel_id).await {
        Some(channel) => channel.guild_id,
        None => {
            
            msg.channel_id
                .say(&ctx.http, "Error finding channel info")
                .await?;

            return Ok(());
        }
    };

    let lava_client = {
        let data = ctx.data.read().await;
        data.get::<Lavalink>().unwrap().clone()
    };

    let manager = songbird::get(ctx).await.unwrap().clone();

    if manager.get(guild_id).is_none() {
        if let Err(ex) = join_interactive(ctx, msg).await {
            msg.channel_id.say(&ctx.http, "Failed to connect to voice channel; maybe I don't have permissions?").await?;
            error!("Failed to connect to vc: {}", ex);
            return Ok(());
        }
    }

    if let Some(_handler) = manager.get(guild_id) {
        
        let query_information = lava_client.auto_search_tracks(&query).await?;

        if query_information.tracks.is_empty() {
            msg.channel_id
                .say(&ctx, "Could not find any video of the search query.")
                .await?;
            return Ok(());
        }

        if let Err(why) = &lava_client.play(guild_id, query_information.tracks[0].clone()).queue()
            .await
        {
            error!("Failed to queue: {}", why);
            return Ok(());
        };

        msg.channel_id
            .say(
                &ctx.http,
                format!(
                    "Added to queue: {}",
                    query_information.tracks[0].info.as_ref().unwrap().title
                ),
            )
            .await?;
    }

    Ok(())
}

#[command]
#[aliases(np, nowplaying)]
async fn now_playing(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let lava_client = data.get::<Lavalink>().unwrap().clone();

    if let Some(node) = lava_client.nodes().await.get(&msg.guild_id.unwrap().0) {
        if let Some(track) = &node.now_playing {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Now Playing: {}", track.track.info.as_ref().unwrap().title),
                )
                .await?;
        } else {
            msg.channel_id.say(&ctx.http, "Nothing is playing at the moment.").await?;
        }
    } else {
        msg.channel_id.say(&ctx.http, "Nothing is playing at the moment.").await?;
    }

    Ok(())
}

#[command]
async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let lava_client = data.get::<Lavalink>().unwrap().clone();

    if let Some(track) = lava_client.skip(msg.guild_id.unwrap()).await {
        msg.channel_id
            .say(
                ctx,
                format!("Skipped: {}", track.track.info.as_ref().unwrap().title),
            )
        .await?;
    } else {
        msg.channel_id.say(&ctx.http, "Nothing to skip.").await?;
    }

    Ok(())
}