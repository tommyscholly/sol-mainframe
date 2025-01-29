# Sol Mainframe

This repository is the monorepo containing a discord bot frontend, which was used by discord members to access their individual progress, and the backend Axum webserver, which was used to store and retrieve the progress of the discord members, as well as update member progress directly from Roblox. 

## Discord Bot Frontend

The primary interface for users to interact with their data was through a discord bot. The bot is written using [Poise](https://github.com/serenity-rs/poise). It supports users retrieving their data, querying the data of other users, and for admins to update the data of users.

## Axum Webserver Backend

The backend is written using the [Axum](https://github.com/tokio-rs/axum) webserver, and Turso's [libsql client](https://docs.rs/libsql/latest/libsql/). The backend is responsible for storing and retrieving user data, as well as updating user data directly from Roblox. 

The backend stores two types of data: user data and game data. Both are stored in SQLite databases, where user data is concerned with what rank the user is (and how much progress they have made towards the next rank), and game data is concerned with what cosmetics the user has unlocked.

## State of the Project

This project was fully functional until it's retirement. The project it was used for changed ownership, and with it, the need for this project was no longer present. The code is provided as-is, and is not actively maintained.
