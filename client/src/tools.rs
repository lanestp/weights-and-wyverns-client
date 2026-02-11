//! MCP tool definitions for the game client.
//!
//! Each tool is a thin pass-through to the game server. The handler
//! sends the command over WebSocket, awaits the response, drains
//! any buffered push events, and returns the combined result.

use std::sync::Arc;

use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::connection::{ConnectionError, GameConnection};
use crate::events::EventBuffer;

// ---------------------------------------------------------------------------
// Parameter types
// ---------------------------------------------------------------------------

/// Parameters for authenticating and joining the game world.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ConnectParams {
    /// Player username.
    pub username: String,
    /// Authentication token.
    pub token: String,
}

/// Parameters for observing the room or examining a target.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LookParams {
    /// Optional target to examine closely.
    pub target: Option<String>,
}

/// Parameters for moving in a direction.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MoveParams {
    /// Direction to move (north, south, east, west, up, down, or exit name).
    pub direction: String,
}

/// Parameters for attacking a target.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AttackParams {
    /// Target to attack.
    pub target: String,
    /// Optional weapon to use.
    pub weapon: Option<String>,
}

/// Parameters for using a class ability.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UseAbilityParams {
    /// Ability name to use.
    pub ability: String,
    /// Optional target for the ability.
    pub target: Option<String>,
}

/// Parameters for picking up an item.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetItemParams {
    /// Item name to pick up.
    pub item: String,
}

/// Parameters for dropping an item.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DropItemParams {
    /// Item name to drop.
    pub item: String,
}

/// Parameters for equipping an item.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct EquipParams {
    /// Item to equip.
    pub item: String,
    /// Optional equipment slot.
    pub slot: Option<String>,
}

/// Parameters for using a consumable item.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UseItemParams {
    /// Consumable item to use.
    pub item: String,
    /// Optional target for the item.
    pub target: Option<String>,
}

/// Parameters for speaking aloud in the room.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SayParams {
    /// Message to speak aloud.
    pub message: String,
}

/// Parameters for sending a private message.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TellParams {
    /// Player name to message.
    pub player: String,
    /// Private message content.
    pub message: String,
}

/// Parameters for broadcasting a zone-wide shout.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ShoutParams {
    /// Message to broadcast to the zone.
    pub message: String,
}

/// Parameters for performing a custom emote.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct EmoteParams {
    /// Custom emote action to perform.
    pub action: String,
}

/// Parameters for sending a message to a chat channel.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ChannelParams {
    /// Channel name (ooc, trade, guild, party).
    pub name: String,
    /// Message to send.
    pub message: String,
}

/// Parameters for inviting a player to a party.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PartyInviteParams {
    /// Player to invite.
    pub player: String,
}

/// Parameters for kicking a player from the party.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PartyKickParams {
    /// Player to kick from the party.
    pub player: String,
}

/// Parameters for auto-matchmaking.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MatchmakeParams {
    /// Preferred role for matchmaking.
    pub role: Option<String>,
    /// Preferred zone for matchmaking.
    pub zone: Option<String>,
}

/// Parameters for commanding the AI companion.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CompanionParams {
    /// Instruction for the AI companion.
    pub command: String,
}

/// Parameters for writing a companion memory note.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CompanionMemoryWriteParams {
    /// The memory note text to save.
    pub text: String,
    /// Optional category tag (e.g., "personality", "relationship", "observation").
    pub tag: Option<String>,
}

/// Parameters for requesting a description suggestion.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SuggestDescriptionParams {
    /// Context or topic for the description.
    pub context: String,
}

/// Parameters for buying an item from a shop.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct BuyParams {
    /// Identifier of the shop to buy from.
    pub shop_id: String,
    /// Name of the item to purchase.
    pub item: String,
}

/// Parameters for selling an item to a shop.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SellParams {
    /// Identifier of the shop to sell to.
    pub shop_id: String,
    /// Name of the item to sell.
    pub item: String,
}

/// Parameters for accepting a quest.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AcceptQuestParams {
    /// Identifier of the quest to accept.
    pub quest_id: String,
}

/// Parameters for completing (turning in) a quest.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CompleteQuestParams {
    /// Identifier of the quest to complete.
    pub quest_id: String,
}

/// Parameters for initiating dialogue with an NPC.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TalkParams {
    /// Name of the NPC to talk to.
    pub target: String,
}

/// Parameters for selecting a dialogue option with an NPC.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DialogueSelectParams {
    /// Name of the NPC in the active dialogue.
    pub npc: String,
    /// Zero-based index of the dialogue option to select.
    pub option: usize,
}

/// Parameters for viewing a leaderboard.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LeaderboardParams {
    /// Type of leaderboard to view (e.g., "level", "gold", "kills"). Defaults to "level".
    pub board_type: Option<String>,
}

/// Parameters for creating a new guild.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GuildCreateParams {
    /// Name for the new guild.
    pub name: String,
}

/// Parameters for inviting a player to a guild.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GuildInviteParams {
    /// Player name to invite.
    pub player: String,
}

/// Parameters for depositing gold into the guild bank.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GuildDepositParams {
    /// Amount of gold to deposit.
    pub amount: u64,
}

// ---------------------------------------------------------------------------
// GameHandler
// ---------------------------------------------------------------------------

/// MCP server handler bridging Claude Code to the game server.
///
/// Holds the WebSocket connection and event buffer behind a mutex
/// so that the rmcp framework can clone and share this handler
/// across async tasks.
#[derive(Clone, Debug)]
pub struct GameHandler {
    connection: Arc<Mutex<GameConnection>>,
    events: Arc<Mutex<EventBuffer>>,
    server_url: String,
    token_path: String,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl GameHandler {
    /// Creates a new handler targeting `server_url` with token storage at `token_path`.
    pub fn new(server_url: String, token_path: String) -> Self {
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            connection: Arc::new(Mutex::new(GameConnection::new(event_tx))),
            events: Arc::new(Mutex::new(EventBuffer::new(event_rx))),
            server_url,
            token_path,
            tool_router: Self::tool_router(),
        }
    }

    // -- Connection tools ---------------------------------------------------

    /// Connect to the game world with username and token. Returns initial room state.
    #[tool(
        description = "Connect to the game world with username and token. Returns initial room state."
    )]
    async fn connect(
        &self,
        Parameters(params): Parameters<ConnectParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let mut conn = self.connection.lock().await;

        if conn.is_connected() {
            conn.disconnect().await;
        }

        conn.connect(&self.server_url)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        // Use provided token, or try reading from per-username token file
        let token = if params.token.is_empty() {
            self.read_token_for(&params.username)
        } else {
            params.token
        };

        let username = params.username.clone();
        let auth_params = serde_json::json!({
            "username": username,
            "token": token,
        });

        drop(conn);
        let result = self.send_and_drain("connect", auth_params).await?;

        // If server returned a new account token, save it to disk
        if let Some(content) = result.content.first() {
            if let Some(text_content) = content.as_text() {
                if let Ok(parsed) = serde_json::from_str::<Value>(&text_content.text) {
                    if let Some(result_obj) = parsed.get("result") {
                        if result_obj.get("new_account").and_then(Value::as_bool) == Some(true) {
                            if let Some(new_token) =
                                result_obj.get("token").and_then(|v| v.as_str())
                            {
                                self.write_token_for(&username, new_token);
                            }
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Disconnect from the game world. Saves your character.
    #[tool(description = "Disconnect from the game world. Saves your character.")]
    async fn disconnect(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        let mut conn = self.connection.lock().await;
        conn.disconnect().await;
        Ok(CallToolResult::success(vec![Content::text(
            r#"{"status":"ok","message":"Disconnected from game server"}"#,
        )]))
    }

    // -- Navigation tools ---------------------------------------------------

    /// Look around the current room, or examine a specific target.
    #[tool(
        description = "Look around the current room, or examine a specific target. Returns room description, exits, players, NPCs, and items."
    )]
    async fn look(
        &self,
        Parameters(params): Parameters<LookParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let mut p = serde_json::Map::new();
        if let Some(target) = params.target {
            p.insert("target".to_owned(), Value::String(target));
        }
        self.send_and_drain("look", Value::Object(p)).await
    }

    /// Move in a direction (north, south, east, west, up, down, or custom exit name).
    #[tool(
        description = "Move in a direction (north, south, east, west, up, down, or custom exit name). Returns the new room state."
    )]
    async fn move_direction(
        &self,
        Parameters(params): Parameters<MoveParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("move", serde_json::json!({ "direction": params.direction }))
            .await
    }

    /// Display a simple ASCII map of nearby explored rooms.
    #[tool(description = "Display a simple ASCII map of nearby explored rooms.")]
    async fn map(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("map", serde_json::json!({})).await
    }

    // -- Combat tools -------------------------------------------------------

    /// Attack a target with your equipped or specified weapon.
    #[tool(
        description = "Attack a target with your equipped or specified weapon. Returns damage dealt and combat state."
    )]
    async fn attack(
        &self,
        Parameters(params): Parameters<AttackParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let mut p = serde_json::json!({ "target": params.target });
        if let Some(weapon) = params.weapon {
            p["weapon"] = Value::String(weapon);
        }
        self.send_and_drain("attack", p).await
    }

    /// Use a class ability, optionally targeting a specific entity.
    #[tool(
        description = "Use a class ability, optionally targeting a specific entity. Returns effect description and state update."
    )]
    async fn use_ability(
        &self,
        Parameters(params): Parameters<UseAbilityParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let mut p = serde_json::json!({ "ability": params.ability });
        if let Some(target) = params.target {
            p["target"] = Value::String(target);
        }
        self.send_and_drain("use_ability", p).await
    }

    /// Attempt to flee from combat.
    #[tool(description = "Attempt to flee from combat. May fail depending on circumstances.")]
    async fn flee(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("flee", serde_json::json!({})).await
    }

    /// Show your full status: HP, mana, level, XP, eq/balance, active effects, and location.
    #[tool(
        description = "Show your full status: HP, mana, level, XP, eq/balance, active effects, and location."
    )]
    async fn status(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("status", serde_json::json!({})).await
    }

    // -- Item tools ---------------------------------------------------------

    /// List all items in your inventory with their stats.
    #[tool(description = "List all items in your inventory with their stats.")]
    async fn inventory(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("inventory", serde_json::json!({}))
            .await
    }

    /// Pick up an item from the current room.
    #[tool(description = "Pick up an item from the current room.")]
    async fn get_item(
        &self,
        Parameters(params): Parameters<GetItemParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("get", serde_json::json!({ "item": params.item }))
            .await
    }

    /// Drop an item from your inventory into the current room.
    #[tool(description = "Drop an item from your inventory into the current room.")]
    async fn drop_item(
        &self,
        Parameters(params): Parameters<DropItemParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("drop", serde_json::json!({ "item": params.item }))
            .await
    }

    /// Equip an item from your inventory.
    #[tool(
        description = "Equip an item from your inventory. Auto-selects the slot if not specified."
    )]
    async fn equip(
        &self,
        Parameters(params): Parameters<EquipParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let mut p = serde_json::json!({ "item": params.item });
        if let Some(slot) = params.slot {
            p["slot"] = Value::String(slot);
        }
        self.send_and_drain("equip", p).await
    }

    /// Use a consumable item (potion, scroll, food).
    #[tool(description = "Use a consumable item (potion, scroll, food), optionally on a target.")]
    async fn use_item(
        &self,
        Parameters(params): Parameters<UseItemParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let mut p = serde_json::json!({ "item": params.item });
        if let Some(target) = params.target {
            p["target"] = Value::String(target);
        }
        self.send_and_drain("use_item", p).await
    }

    // -- Social tools -------------------------------------------------------

    /// Say something aloud in the current room.
    #[tool(
        description = "Say something aloud in the current room. All present players will see it."
    )]
    async fn say(
        &self,
        Parameters(params): Parameters<SayParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("say", serde_json::json!({ "message": params.message }))
            .await
    }

    /// Send a private message to a specific player.
    #[tool(description = "Send a private message to a specific player.")]
    async fn tell(
        &self,
        Parameters(params): Parameters<TellParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain(
            "tell",
            serde_json::json!({ "player": params.player, "message": params.message }),
        )
        .await
    }

    /// Shout a message to the entire zone.
    #[tool(description = "Shout a message to the entire zone. Costs gold.")]
    async fn shout(
        &self,
        Parameters(params): Parameters<ShoutParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("shout", serde_json::json!({ "message": params.message }))
            .await
    }

    /// Perform a custom emote visible to the room.
    #[tool(description = "Perform a custom emote visible to the room (e.g., 'dances a jig').")]
    async fn emote(
        &self,
        Parameters(params): Parameters<EmoteParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("emote", serde_json::json!({ "action": params.action }))
            .await
    }

    /// List all online players with their level and location.
    #[tool(description = "List all online players with their level and location.")]
    async fn who(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("who", serde_json::json!({})).await
    }

    /// Send a message to a chat channel (ooc, trade, guild, party).
    #[tool(description = "Send a message to a chat channel (ooc, trade, guild, party).")]
    async fn channel(
        &self,
        Parameters(params): Parameters<ChannelParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain(
            "channel",
            serde_json::json!({ "name": params.name, "message": params.message }),
        )
        .await
    }

    /// Initiate dialogue with an NPC in the current room.
    #[tool(
        description = "Talk to an NPC in the current room. Returns dialogue text and available response options."
    )]
    async fn talk(
        &self,
        Parameters(params): Parameters<TalkParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("talk", serde_json::json!({ "target": params.target }))
            .await
    }

    /// Select a dialogue option in an active NPC conversation.
    #[tool(
        description = "Select a dialogue option in an active NPC conversation. Provide the NPC name and the zero-based index of the option."
    )]
    async fn dialogue_select(
        &self,
        Parameters(params): Parameters<DialogueSelectParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain(
            "dialogue_select",
            serde_json::json!({ "npc": params.npc, "option": params.option }),
        )
        .await
    }

    // -- Party tools --------------------------------------------------------

    /// Invite another player to join your party.
    #[tool(description = "Invite another player to join your party.")]
    async fn party_invite(
        &self,
        Parameters(params): Parameters<PartyInviteParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain(
            "party_invite",
            serde_json::json!({ "player": params.player }),
        )
        .await
    }

    /// Accept a pending party invitation.
    #[tool(description = "Accept a pending party invitation.")]
    async fn party_accept(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("party_accept", serde_json::json!({}))
            .await
    }

    /// Leave your current party.
    #[tool(description = "Leave your current party.")]
    async fn party_leave(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("party_leave", serde_json::json!({}))
            .await
    }

    /// Kick a member from your party (leader only).
    #[tool(description = "Kick a member from your party. Only the party leader can do this.")]
    async fn party_kick(
        &self,
        Parameters(params): Parameters<PartyKickParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("party_kick", serde_json::json!({ "player": params.player }))
            .await
    }

    /// Show party members with their HP and location.
    #[tool(description = "Show party members with their HP and location.")]
    async fn party_list(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("party_list", serde_json::json!({}))
            .await
    }

    /// Queue for auto-matchmaking.
    #[tool(
        description = "Queue for auto-matchmaking. Optionally specify a preferred role and/or zone."
    )]
    async fn matchmake(
        &self,
        Parameters(params): Parameters<MatchmakeParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let mut p = serde_json::Map::new();
        if let Some(role) = params.role {
            p.insert("role".to_owned(), Value::String(role));
        }
        if let Some(zone) = params.zone {
            p.insert("zone".to_owned(), Value::String(zone));
        }
        self.send_and_drain("matchmake", Value::Object(p)).await
    }

    // -- Companion tools ----------------------------------------------------

    /// Command your AI companion.
    #[tool(
        description = "Command your AI companion (e.g., 'guard the door', 'heal me when below half HP', 'scout ahead')."
    )]
    async fn companion(
        &self,
        Parameters(params): Parameters<CompanionParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain(
            "companion",
            serde_json::json!({ "command": params.command }),
        )
        .await
    }

    /// Check your AI companion's HP, equipment, and current behavior.
    #[tool(description = "Check your AI companion's HP, equipment, and current behavior.")]
    async fn companion_status(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("companion_status", serde_json::json!({}))
            .await
    }

    /// Read your companion's core memories.
    #[tool(
        description = "Read your companion's core memories — milestones and notes from your journey together."
    )]
    async fn companion_memory(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("companion_memory", serde_json::json!({}))
            .await
    }

    /// Save a core memory note for your companion.
    #[tool(
        description = "Save a core memory note for your companion. Use this to record important observations, personality traits, or relationship notes that should persist across sessions."
    )]
    async fn companion_memory_write(
        &self,
        Parameters(params): Parameters<CompanionMemoryWriteParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let mut p = serde_json::json!({ "text": params.text });
        if let Some(tag) = params.tag {
            p["tag"] = Value::String(tag);
        }
        self.send_and_drain("companion_memory_write", p).await
    }

    // -- Character tools ----------------------------------------------------

    /// View your full character sheet.
    #[tool(
        description = "View your full character sheet: class, level, stats, abilities, and equipment."
    )]
    async fn character_info(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("character_info", serde_json::json!({}))
            .await
    }

    /// List all your available abilities with descriptions and cooldowns.
    #[tool(description = "List all your available abilities with descriptions and cooldowns.")]
    async fn abilities(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("abilities", serde_json::json!({}))
            .await
    }

    /// Show your active and completed quests.
    #[tool(description = "Show your active and completed quests.")]
    async fn quests(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("quests", serde_json::json!({})).await
    }

    /// View a leaderboard ranking.
    #[tool(
        description = "View a leaderboard ranking. Board types include 'level', 'gold', 'kills', etc. Defaults to 'level'."
    )]
    async fn leaderboard(
        &self,
        Parameters(params): Parameters<LeaderboardParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let board_type = params.board_type.unwrap_or_else(|| "level".to_owned());
        self.send_and_drain(
            "leaderboard",
            serde_json::json!({ "board_type": board_type }),
        )
        .await
    }

    /// Get a description suggestion for a given context.
    #[tool(description = "Get the server's suggested description for a given context or topic.")]
    async fn suggest_description(
        &self,
        Parameters(params): Parameters<SuggestDescriptionParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain(
            "suggest_description",
            serde_json::json!({ "context": params.context }),
        )
        .await
    }

    // -- Shop tools ---------------------------------------------------------

    /// Buy an item from a shop.
    #[tool(description = "Buy an item from a shop. Requires gold.")]
    async fn buy(
        &self,
        Parameters(params): Parameters<BuyParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain(
            "buy",
            serde_json::json!({ "shop_id": params.shop_id, "item": params.item }),
        )
        .await
    }

    /// Sell an item to a shop for gold.
    #[tool(description = "Sell an item to a shop for gold.")]
    async fn sell(
        &self,
        Parameters(params): Parameters<SellParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain(
            "sell",
            serde_json::json!({ "shop_id": params.shop_id, "item": params.item }),
        )
        .await
    }

    // -- Quest tools --------------------------------------------------------

    /// Accept a quest from an NPC.
    #[tool(description = "Accept a quest from an NPC. Adds it to your quest log.")]
    async fn accept_quest(
        &self,
        Parameters(params): Parameters<AcceptQuestParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain(
            "accept_quest",
            serde_json::json!({ "quest_id": params.quest_id }),
        )
        .await
    }

    /// Turn in a completed quest for rewards.
    #[tool(description = "Turn in a completed quest for rewards.")]
    async fn complete_quest(
        &self,
        Parameters(params): Parameters<CompleteQuestParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain(
            "complete_quest",
            serde_json::json!({ "quest_id": params.quest_id }),
        )
        .await
    }

    // -- Guild tools --------------------------------------------------------

    /// Create a new guild with the given name.
    #[tool(description = "Create a new guild. You become the guild leader.")]
    async fn guild_create(
        &self,
        Parameters(params): Parameters<GuildCreateParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("guild_create", serde_json::json!({ "name": params.name }))
            .await
    }

    /// Invite another player to join your guild.
    #[tool(description = "Invite another player to join your guild.")]
    async fn guild_invite(
        &self,
        Parameters(params): Parameters<GuildInviteParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain(
            "guild_invite",
            serde_json::json!({ "player": params.player }),
        )
        .await
    }

    /// Leave your current guild.
    #[tool(description = "Leave your current guild.")]
    async fn guild_leave(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("guild_leave", serde_json::json!({}))
            .await
    }

    /// View information about your guild.
    #[tool(description = "View your guild's information: members, bank, and rank.")]
    async fn guild_info(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain("guild_info", serde_json::json!({}))
            .await
    }

    /// Deposit gold into the guild bank.
    #[tool(description = "Deposit gold into the guild bank.")]
    async fn guild_deposit(
        &self,
        Parameters(params): Parameters<GuildDepositParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.send_and_drain(
            "guild_deposit",
            serde_json::json!({ "amount": params.amount }),
        )
        .await
    }
}

// ---------------------------------------------------------------------------
// ServerHandler implementation
// ---------------------------------------------------------------------------

#[tool_handler]
impl ServerHandler for GameHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Weights & Wyverns game client. Use these tools to explore, fight, \
                 and interact in the multiplayer text dungeon."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

impl GameHandler {
    /// Sends a command to the game server, awaits the response, drains
    /// buffered events, and returns the combined JSON as an MCP tool result.
    async fn send_and_drain(
        &self,
        action: &str,
        params: Value,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let response = {
            let conn = self.connection.lock().await;
            conn.send_command(action, params).await
        };

        let response = response.map_err(|e| match &e {
            ConnectionError::NotConnected => rmcp::ErrorData::invalid_request(
                "Not connected to game server — call `connect` first",
                None,
            ),
            ConnectionError::Timeout(_) => {
                rmcp::ErrorData::internal_error(format!("Server timed out: {e}"), None)
            }
            _ => rmcp::ErrorData::internal_error(e.to_string(), None),
        })?;

        let events = {
            let mut event_buffer = self.events.lock().await;
            event_buffer.drain()
        };

        let combined = serde_json::json!({
            "result": response,
            "events": events,
        });

        Ok(CallToolResult::success(vec![Content::text(
            combined.to_string(),
        )]))
    }

    /// Reads the token for a specific username, returning empty string if unavailable.
    ///
    /// Tokens are stored per-username at `<token_path>/tokens/<username>`.
    /// Falls back to the legacy single-file `<token_path>/token` if the
    /// per-username file doesn't exist (one-time migration path).
    fn read_token_for(&self, username: &str) -> String {
        let base = expand_tilde(&self.token_path);
        let base = std::path::Path::new(&base);

        // Try per-username path first
        let per_user = base.join("tokens").join(username);
        if let Ok(contents) = std::fs::read_to_string(&per_user) {
            return contents.trim().to_owned();
        }

        // Fall back to legacy single-file path for migration
        let legacy = base.join("token");
        if let Ok(contents) = std::fs::read_to_string(&legacy) {
            let token = contents.trim().to_owned();
            if !token.is_empty() {
                tracing::info!(username, "token.migrating_legacy_file");
                // Migrate: write to per-username, delete legacy
                self.write_token_for(username, &token);
                let _ = std::fs::remove_file(&legacy);
            }
            return token;
        }

        String::new()
    }

    /// Writes a token for a specific username, creating directories as needed.
    ///
    /// Tokens are stored at `<token_path>/tokens/<username>`.
    fn write_token_for(&self, username: &str, token: &str) {
        let base = expand_tilde(&self.token_path);
        let base = std::path::Path::new(&base);
        let tokens_dir = base.join("tokens");
        if let Err(err) = std::fs::create_dir_all(&tokens_dir) {
            tracing::warn!(error = %err, "token.dir.create.failed");
            return;
        }
        let path = tokens_dir.join(username);
        if let Err(err) = std::fs::write(&path, token) {
            tracing::warn!(error = %err, username, "token.file.write.failed");
        }
    }
}

/// Expands a leading `~` in a path to the user's home directory.
fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return format!("{}/{rest}", home.to_string_lossy());
        }
    }
    path.to_owned()
}
