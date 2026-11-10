//! What is asked of an AI service, and how its answers are read. Pure: this
//! builds request bodies and reads response bodies; the key, the network and
//! the switches live in `app::ai`.
//!
//! AI is a choice here. Nothing in this module runs unless someone has given
//! the app a key, switched AI on, left the feature's own switch on, and then
//! asked: there is no request the app makes on its own. And it is built to
//! cost next to nothing: the smallest model of whichever service was chosen,
//! a hard cap on every answer, and nothing spent on thinking where the
//! service lets that be said.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::settings::Provider;

/// The Anthropic Messages API version sent with every request to it.
pub const ANTHROPIC_VERSION: &str = "2026-10-21";

/// The most a request may be answered with, per kind of answer: a line, a
/// few sentences, or one small JSON object. It is all an answer can cost.
pub const MAX_TOKENS_LINE: u32 = 120;
/// See [`MAX_TOKENS_LINE`].
pub const MAX_TOKENS_WORDS: u32 = 300;
/// See [`MAX_TOKENS_LINE`].
pub const MAX_TOKENS_PLAN: u32 = 200;

/// The longest digest sent. Longer is refused, never cut: a month of days
/// is about three thousand characters, so more means something is wrong,
/// and half a month would be read as a whole one. It is also the ceiling on
/// what one request can cost to send.
pub const MAX_INPUT_CHARS: usize = 6_000;

/// The longest question or sentence someone may type.
pub const MAX_QUESTION_CHARS: usize = 300;

/// The model asked when none is named in Settings: the smallest and
/// cheapest each service documented when this was written. A service that
/// is not one of the two named has no default; its model must be named.
pub fn default_model(provider: Provider) -> &'static str {
    match provider {
        Provider::Anthropic => "claude-haiku-5-5",
        Provider::OpenAi => "gpt-5.4-nano",
        Provider::Custom => "",
    }
}

/// Why no answer came back, in terms a person can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiError {
    /// No API key has been given to the app for this service.
    NoKey,
    /// AI, or this feature, is switched off.
    Off,
    /// The service turned the key down.
    BadKey,
    /// Rate limited or overloaded; trying again later may work.
    Busy,
    /// The request was declined by the model.
    Declined,
    /// What was to be sent is longer than its cap.
    TooLong,
    /// Nothing usable came back.
    Empty,
    /// A service of one's own was chosen and no model was named for it.
    NoModel,
    /// The address of a service of one's own is not one the app will send to.
    BadAddress,
    /// The service does not know the model named.
    UnknownModel,
    /// The network, or the service, failed in some other way.
    Failed(String),
}

impl AiError {
    /// The sentence the window shows.
    pub fn sentence(&self) -> String {
        match self {
            Self::NoKey => "AI is locked: no API key has been added.".to_owned(),
            Self::Off => "AI is switched off.".to_owned(),
            Self::BadKey => "The service did not accept that API key.".to_owned(),
            Self::Busy => "The service is busy just now. Try again in a minute.".to_owned(),
            Self::Declined => "The model declined to answer that.".to_owned(),
            Self::TooLong => "That is too long to send.".to_owned(),
            Self::Empty => "Nothing came back.".to_owned(),
            Self::NoModel => "Name a model for this service in Settings.".to_owned(),
            Self::BadAddress => {
                "The service's address must start with https://, or be this machine.".to_owned()
            }
            Self::UnknownModel => {
                "The service does not know that model. Check its name in Settings.".to_owned()
            }
            Self::Failed(why) => format!("The service could not be reached: {why}"),
        }
    }

    /// The error for an HTTP status that is not a success.
    pub fn from_status(status: u16) -> Self {
        match status {
            401 | 403 => Self::BadKey,
            404 => Self::UnknownModel,
            429 | 529 => Self::Busy,
            other => Self::Failed(format!("HTTP {other}")),
        }
    }
}

/// One thing asked, whichever service it goes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    /// How to answer.
    pub system: &'static str,
    /// What is sent: the days, the question, the sentence.
    pub content: String,
    /// The cap on the answer.
    pub max_tokens: u32,
    /// The answer is to be the plan's JSON object.
    pub plan: bool,
}

/// How one day is to be put into a line.
const DAY_SYSTEM: &str = "You read one day from a personal almanac kept by a small desktop app: \
its hours at the keyboard, the first and last touch, the time outside working hours, and the \
night that led into it. Say what the day was in one plain sentence of at most twenty words, to \
the person whose day it is, addressed as you. Use the times and figures given and no others. \
Say night or quiet, never that they slept. No advice, no judgement, no preamble.";

/// How the week is to be put into words.
const WEEK_SYSTEM: &str = "You read one week from a personal almanac kept by a small desktop app. \
Each line is a day: its hours at the keyboard, the first and last touch, the time outside \
working hours, and the night that led into it (the quiet between the last touch of one day and \
the first of the next). Write what this week was like in two or three plain sentences, for the \
person whose week it is, addressed as you. Say what stands out: the long day, the short night, \
the day off, hours that crept later. Use the times and figures given and no others. The app \
knows when the keys went quiet, not what the person did, so say night or quiet, never that they \
slept, and do not give advice or judge. No lists, no headings, no preamble.";

/// How a question about the almanac is to be answered.
const ASK_SYSTEM: &str = "You answer one question about a personal almanac kept by a small desktop \
app. You are given some days, one line each: hours at the keyboard, first and last touch, time \
outside working hours, the night that led into the day, and whether it was leave. Answer from \
those lines only, in one or two plain sentences, addressed as you, working out any totals or \
averages yourself. If the lines do not hold the answer, say so in a few words. Say night or \
quiet, never that they slept. No advice, no preamble.";

/// How a sentence is to be turned into a plan. The shape is spelled out in
/// words as well as in a schema, because not every service takes a schema.
const PLAN_SYSTEM: &str = "You turn one sentence into a plan for an app that keeps a laptop awake. \
The app can do four things: keep it awake for a length of time, keep it awake until a time of \
day, keep it awake until told otherwise, or let it sleep. Read the sentence and choose one. \
Reply with only a JSON object with exactly these keys: \"kind\", one of \"for\", \"until\", \
\"indefinite\", \"off\" or \"unclear\"; \"minutes\", a whole number, the length for \"for\" and 0 \
otherwise; \"time\", for \"until\" the time of day as 24-hour HH:MM in the person's local time \
and an empty string otherwise; and \"say\", the plan in a few plain words as the app would say \
it, such as 'Awake for 2 hours' or 'Awake until 6:30 pm', or for \"unclear\" what was not \
understood. The current local time is given so that words like tonight, noon or in the morning \
can be resolved. If the sentence asks for something else or you cannot tell, choose \"unclear\".";

/// A day put into a line. `digest` is that day's line.
pub fn day_ask(digest: &str) -> Result<Ask, AiError> {
    fits(digest, MAX_INPUT_CHARS)?;
    Ok(Ask {
        system: DAY_SYSTEM,
        content: digest.to_owned(),
        max_tokens: MAX_TOKENS_LINE,
        plan: false,
    })
}

/// A week put into words. `digest` is the week as the window lays it out,
/// one day a line.
pub fn week_ask(digest: &str) -> Result<Ask, AiError> {
    fits(digest, MAX_INPUT_CHARS)?;
    Ok(Ask {
        system: WEEK_SYSTEM,
        content: digest.to_owned(),
        max_tokens: MAX_TOKENS_WORDS,
        plan: false,
    })
}

/// `question`, answered from `digest`, the days in view.
pub fn question_ask(question: &str, digest: &str) -> Result<Ask, AiError> {
    fits(question, MAX_QUESTION_CHARS)?;
    fits(digest, MAX_INPUT_CHARS)?;
    Ok(Ask {
        system: ASK_SYSTEM,
        content: format!("{digest}\n\nQuestion: {question}"),
        max_tokens: MAX_TOKENS_WORDS,
        plan: false,
    })
}

/// `sentence`, turned into a plan. `now_local` is the local date and time
/// as the window has it, `Tue 10 Nov 2026, 18:42`.
pub fn plan_ask(sentence: &str, now_local: &str) -> Result<Ask, AiError> {
    fits(sentence, MAX_QUESTION_CHARS)?;
    Ok(Ask {
        system: PLAN_SYSTEM,
        content: format!("It is now {now_local}.\n\n{sentence}"),
        max_tokens: MAX_TOKENS_PLAN,
        plan: true,
    })
}

fn fits(text: &str, most: usize) -> Result<(), AiError> {
    if text.trim().is_empty() {
        return Err(AiError::Empty);
    }
    if text.chars().count() > most {
        return Err(AiError::TooLong);
    }
    Ok(())
}

/// The shape a plan comes back in.
fn plan_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "kind": {
                "type": "string",
                "enum": ["for", "until", "indefinite", "off", "unclear"],
            },
            "minutes": { "type": "integer" },
            "time": { "type": "string" },
            "say": { "type": "string" },
        },
        "required": ["kind", "minutes", "time", "say"],
        "additionalProperties": false,
    })
}

/// The model to ask: the one named in Settings, else the service's default.
/// A service of one's own has no default.
pub fn model_for(provider: Provider, named: &str) -> Result<String, AiError> {
    let named = named.trim();
    let model = if named.is_empty() {
        default_model(provider)
    } else {
        named
    };
    if model.is_empty() {
        Err(AiError::NoModel)
    } else {
        Ok(model.to_owned())
    }
}

/// The request body for `ask`, in the shape `provider` takes.
///
/// Anthropic: the Messages API, with thinking switched off and effort low,
/// so that no token is spent on anything but the answer. `OpenAI`: Chat
/// Completions, the instructions as a developer message, the answer capped
/// with `max_completion_tokens`, a plan held to its schema. A service of
/// one's own: the widely shared older form of the same, with a system
/// message and `max_tokens`, and no schema, since many do not take one.
pub fn body(provider: Provider, model: &str, ask: &Ask) -> Value {
    match provider {
        Provider::Anthropic => {
            let mut body = json!({
                "model": model,
                "max_tokens": ask.max_tokens,
                "thinking": { "type": "disabled" },
                "output_config": { "effort": "low" },
                "system": ask.system,
                "messages": [{ "role": "user", "content": ask.content }],
            });
            if ask.plan {
                body["output_config"]["format"] = json!({
                    "type": "json_schema",
                    "schema": plan_schema(),
                });
            }
            body
        }
        Provider::OpenAi => {
            let mut body = json!({
                "model": model,
                "max_completion_tokens": ask.max_tokens,
                "messages": [
                    { "role": "developer", "content": ask.system },
                    { "role": "user", "content": ask.content },
                ],
            });
            if ask.plan {
                body["response_format"] = json!({
                    "type": "json_schema",
                    "json_schema": { "name": "plan", "strict": true, "schema": plan_schema() },
                });
            }
            body
        }
        Provider::Custom => json!({
            "model": model,
            "max_tokens": ask.max_tokens,
            "messages": [
                { "role": "system", "content": ask.system },
                { "role": "user", "content": ask.content },
            ],
        }),
    }
}

/// Where requests for `provider` go. For a service of one's own, `base` is
/// its address (`https://openrouter.ai/api/v1`), which must be `https`, or
/// plain `http` only to this machine, so a key is never sent in the clear
/// across a network.
pub fn endpoint(provider: Provider, base: &str) -> Result<String, AiError> {
    match provider {
        Provider::Anthropic => Ok("https://api.anthropic.com/v1/messages".to_owned()),
        Provider::OpenAi => Ok("https://api.openai.com/v1/chat/completions".to_owned()),
        Provider::Custom => {
            let base = base.trim().trim_end_matches('/');
            let local = ["http://localhost", "http://127.0.0.1", "http://[::1]"]
                .iter()
                .any(|here| {
                    base.strip_prefix(here)
                        .is_some_and(|rest| rest.is_empty() || rest.starts_with([':', '/']))
                });
            let secure = base
                .strip_prefix("https://")
                .is_some_and(|rest| !rest.is_empty() && !rest.starts_with('/'));
            if !(secure || local) || base.contains(char::is_whitespace) {
                return Err(AiError::BadAddress);
            }
            Ok(format!("{base}/chat/completions"))
        }
    }
}

/// The words of a response. A response the model declined, or one with no
/// text in it, is an error, checked before the content is read.
pub fn text_of(provider: Provider, response: &Value) -> Result<String, AiError> {
    let text = match provider {
        Provider::Anthropic => {
            if response["stop_reason"] == "refusal" {
                return Err(AiError::Declined);
            }
            let blocks: Vec<&str> = response["content"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|block| block["type"] == "text")
                .filter_map(|block| block["text"].as_str())
                .collect();
            blocks.join("")
        }
        Provider::OpenAi | Provider::Custom => {
            let choice = &response["choices"][0];
            let refused = choice["message"]["refusal"]
                .as_str()
                .is_some_and(|why| !why.is_empty());
            if refused || choice["finish_reason"] == "content_filter" {
                return Err(AiError::Declined);
            }
            choice["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_owned()
        }
    };
    let text = text.trim().to_owned();
    if text.is_empty() {
        Err(AiError::Empty)
    } else {
        Ok(text)
    }
}

/// What kind of plan a sentence was understood as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Understanding {
    /// Awake for `minutes`.
    For,
    /// Awake until `time`.
    Until,
    /// Awake until told otherwise.
    Indefinite,
    /// Let it sleep.
    Off,
    /// Not something the app can do, or not clear enough to act on.
    Unclear,
}

/// A sentence, understood. The window shows `say` and asks before it acts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Understood {
    /// What kind of plan.
    pub kind: Understanding,
    /// For `for`: how long, in minutes. Zero otherwise.
    pub minutes: u32,
    /// For `until`: the local time of day, `18:30`. Empty otherwise.
    pub time: String,
    /// The plan in a few plain words, or what was not understood.
    pub say: String,
}

/// A week is the longest the app will keep a laptop awake for on a timer.
const MAX_MINUTES: i64 = 7 * 24 * 60;

/// Reads the plan out of a response. A service that takes no schema may
/// wrap the object in a sentence or a code fence, so the object is looked
/// for inside the text. Anything that does not hold together (a length of
/// no minutes, a time that is not one) reads as unclear, so the window
/// never acts on half an answer.
pub fn understood(provider: Provider, response: &Value) -> Result<Understood, AiError> {
    let text = text_of(provider, response)?;
    let object = match (text.find('{'), text.rfind('}')) {
        (Some(from), Some(to)) if from < to => &text[from..=to],
        _ => return Err(AiError::Empty),
    };
    let raw: Value = serde_json::from_str(object).map_err(|_| AiError::Empty)?;
    let kind: Understanding =
        serde_json::from_value(raw["kind"].clone()).map_err(|_| AiError::Empty)?;
    let minutes = raw["minutes"].as_i64().unwrap_or(0);
    let time = raw["time"].as_str().unwrap_or("").trim().to_owned();
    let say = raw["say"].as_str().unwrap_or("").trim().to_owned();
    let holds = match kind {
        Understanding::For => (1..=MAX_MINUTES).contains(&minutes),
        Understanding::Until => is_time(&time),
        _ => true,
    };
    let kind = if holds { kind } else { Understanding::Unclear };
    Ok(Understood {
        kind,
        minutes: if kind == Understanding::For {
            u32::try_from(minutes).unwrap_or(0)
        } else {
            0
        },
        time: if kind == Understanding::Until {
            time
        } else {
            String::new()
        },
        say,
    })
}

/// `HH:MM`, 24-hour.
fn is_time(time: &str) -> bool {
    let Some((hours, minutes)) = time.split_once(':') else {
        return false;
    };
    let in_range = |part: &str, max: u8| {
        (1..=2).contains(&part.len()) && part.parse::<u8>().is_ok_and(|n| n <= max)
    };
    in_range(hours, 23) && minutes.len() == 2 && in_range(minutes, 59)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Provider; 3] = [Provider::Anthropic, Provider::OpenAi, Provider::Custom];

    fn claude(text: &str) -> Value {
        json!({
            "stop_reason": "end_turn",
            "content": [{ "type": "text", "text": text }],
        })
    }

    fn chat(text: &str) -> Value {
        json!({
            "choices": [{
                "finish_reason": "stop",
                "message": { "role": "assistant", "content": text, "refusal": null },
            }],
        })
    }

    fn asks() -> Vec<Ask> {
        vec![
            day_ask("Mon 9 Nov: 10h 56m").unwrap(),
            week_ask("Mon 9 Nov: 10h 56m").unwrap(),
            question_ask("How long?", "Mon 9 Nov: 10h 56m").unwrap(),
            plan_ask("two hours", "now").unwrap(),
        ]
    }

    #[test]
    fn every_answer_is_capped_at_a_few_sentences() {
        let caps: Vec<u32> = asks().iter().map(|ask| ask.max_tokens).collect();
        assert_eq!(
            caps,
            [
                MAX_TOKENS_LINE,
                MAX_TOKENS_WORDS,
                MAX_TOKENS_WORDS,
                MAX_TOKENS_PLAN
            ]
        );
        assert!(caps.iter().all(|cap| *cap <= 300));
    }

    #[test]
    fn a_request_to_claude_is_as_cheap_as_one_can_be() {
        for ask in asks() {
            let body = body(Provider::Anthropic, "claude-haiku-5-5", &ask);
            assert_eq!(body["model"], "claude-haiku-5-5");
            assert_eq!(
                body["thinking"]["type"], "disabled",
                "nothing spent thinking"
            );
            assert_eq!(body["output_config"]["effort"], "low");
            assert_eq!(body["max_tokens"], ask.max_tokens);
            assert_eq!(body["system"], ask.system);
            assert_eq!(body["messages"][0]["content"], ask.content.as_str());
            assert!(body.get("temperature").is_none(), "rejected by this model");
            assert!(
                body.get("fallbacks").is_none(),
                "no second model, no second bill"
            );
        }
        let plan = body(
            Provider::Anthropic,
            "claude-haiku-5-5",
            &plan_ask("two hours", "now").unwrap(),
        );
        assert_eq!(plan["output_config"]["format"]["type"], "json_schema");
        assert_eq!(
            plan["output_config"]["format"]["schema"]["additionalProperties"],
            false
        );
    }

    #[test]
    fn a_request_to_openai_caps_the_answer_and_holds_a_plan_to_its_shape() {
        let week = body(
            Provider::OpenAi,
            "gpt-5.4-nano",
            &week_ask("Mon: 8h").unwrap(),
        );
        assert_eq!(week["model"], "gpt-5.4-nano");
        assert_eq!(week["max_completion_tokens"], MAX_TOKENS_WORDS);
        assert!(week.get("max_tokens").is_none(), "deprecated there");
        assert_eq!(week["messages"][0]["role"], "developer");
        assert_eq!(week["messages"][1]["role"], "user");
        assert_eq!(week["messages"][1]["content"], "Mon: 8h");
        assert!(week.get("response_format").is_none());

        let plan = body(
            Provider::OpenAi,
            "gpt-5.4-nano",
            &plan_ask("two hours", "now").unwrap(),
        );
        let format = &plan["response_format"];
        assert_eq!(format["type"], "json_schema");
        assert_eq!(format["json_schema"]["name"], "plan");
        assert_eq!(format["json_schema"]["strict"], true);
        assert_eq!(
            format["json_schema"]["schema"]["required"],
            json!(["kind", "minutes", "time", "say"])
        );
    }

    #[test]
    fn a_request_to_another_service_keeps_to_what_most_of_them_take() {
        let plan = body(
            Provider::Custom,
            "llama3.2",
            &plan_ask("two hours", "now").unwrap(),
        );
        assert_eq!(plan["model"], "llama3.2");
        assert_eq!(plan["max_tokens"], MAX_TOKENS_PLAN);
        assert_eq!(plan["messages"][0]["role"], "system");
        assert!(plan.get("response_format").is_none(), "many take no schema");
        assert!(
            plan["messages"][0]["content"]
                .as_str()
                .unwrap()
                .contains("Reply with only a JSON object"),
            "so the shape is asked for in words"
        );
    }

    #[test]
    fn the_smallest_model_is_the_default_and_a_named_one_is_used_as_named() {
        assert_eq!(
            model_for(Provider::Anthropic, "").as_deref(),
            Ok("claude-haiku-5-5")
        );
        assert_eq!(
            model_for(Provider::OpenAi, "  ").as_deref(),
            Ok("gpt-5.4-nano")
        );
        assert_eq!(
            model_for(Provider::OpenAi, " gpt-5-nano ").as_deref(),
            Ok("gpt-5-nano")
        );
        assert_eq!(model_for(Provider::Custom, ""), Err(AiError::NoModel));
        assert_eq!(
            model_for(Provider::Custom, "llama3.2").as_deref(),
            Ok("llama3.2")
        );
    }

    #[test]
    fn requests_go_to_each_service_and_a_key_is_never_sent_in_the_clear() {
        assert_eq!(
            endpoint(Provider::Anthropic, "ignored").as_deref(),
            Ok("https://api.anthropic.com/v1/messages")
        );
        assert_eq!(
            endpoint(Provider::OpenAi, "").as_deref(),
            Ok("https://api.openai.com/v1/chat/completions")
        );
        assert_eq!(
            endpoint(Provider::Custom, " https://openrouter.ai/api/v1/ ").as_deref(),
            Ok("https://openrouter.ai/api/v1/chat/completions")
        );
        assert_eq!(
            endpoint(Provider::Custom, "http://localhost:11434/v1").as_deref(),
            Ok("http://localhost:11434/v1/chat/completions")
        );
        assert!(endpoint(Provider::Custom, "http://127.0.0.1:8080/v1").is_ok());
        for bad in [
            "",
            "openrouter.ai/api/v1",
            "http://example.com/v1",
            "http://localhost.example.com/v1",
            "https://",
            "ftp://example.com",
            "https://exa mple.com/v1",
        ] {
            assert_eq!(
                endpoint(Provider::Custom, bad),
                Err(AiError::BadAddress),
                "{bad}"
            );
        }
    }

    #[test]
    fn nothing_and_too_much_are_refused_before_anything_is_sent() {
        assert_eq!(week_ask("  "), Err(AiError::Empty));
        let long = "x".repeat(MAX_INPUT_CHARS + 1);
        assert_eq!(week_ask(&long), Err(AiError::TooLong));
        assert_eq!(day_ask(&long), Err(AiError::TooLong));
        assert!(week_ask(&"x".repeat(MAX_INPUT_CHARS)).is_ok());
        let wordy = "x".repeat(MAX_QUESTION_CHARS + 1);
        assert_eq!(plan_ask(&wordy, "now"), Err(AiError::TooLong));
        assert_eq!(question_ask(&wordy, "Mon: 8h"), Err(AiError::TooLong));
        assert_eq!(question_ask("How long?", &long), Err(AiError::TooLong));
        assert_eq!(question_ask(" ", "Mon: 8h"), Err(AiError::Empty));
    }

    #[test]
    fn what_is_sent_is_the_days_then_the_question_and_how_to_speak_of_them() {
        let ask = question_ask("How many hours?", "Mon 9 Nov: 8h\nTue 10 Nov: 7h").unwrap();
        assert_eq!(
            ask.content,
            "Mon 9 Nov: 8h\nTue 10 Nov: 7h\n\nQuestion: How many hours?"
        );
        assert!(ask.system.contains("those lines only"));
        let week = week_ask("Mon: 8h").unwrap();
        assert!(week.system.contains("never that they"), "night, not sleep");
        let plan = plan_ask("two hours please", "Tue 10 Nov 2026, 18:42").unwrap();
        assert!(
            plan.content
                .starts_with("It is now Tue 10 Nov 2026, 18:42.")
        );
        assert!(plan.content.ends_with("two hours please"));
    }

    #[test]
    fn the_words_are_read_from_each_services_shape() {
        assert_eq!(
            text_of(Provider::Anthropic, &claude(" A long Monday. ")).unwrap(),
            "A long Monday."
        );
        for provider in [Provider::OpenAi, Provider::Custom] {
            assert_eq!(
                text_of(provider, &chat(" A long Monday. ")).unwrap(),
                "A long Monday."
            );
        }
        for provider in ALL {
            assert_eq!(text_of(provider, &json!({})), Err(AiError::Empty));
        }
    }

    #[test]
    fn a_refusal_is_not_read_as_an_answer() {
        let declined = json!({
            "stop_reason": "refusal",
            "content": [{ "type": "text", "text": "partial" }],
        });
        assert_eq!(
            text_of(Provider::Anthropic, &declined),
            Err(AiError::Declined)
        );
        let refused = json!({
            "choices": [{
                "finish_reason": "stop",
                "message": { "content": null, "refusal": "I can't help with that." },
            }],
        });
        assert_eq!(text_of(Provider::OpenAi, &refused), Err(AiError::Declined));
        let filtered = json!({
            "choices": [{ "finish_reason": "content_filter", "message": { "content": "x" } }],
        });
        assert_eq!(text_of(Provider::Custom, &filtered), Err(AiError::Declined));
    }

    #[test]
    fn a_plan_is_read_out_of_the_json_bare_or_wrapped() {
        let json = r#"{"kind":"for","minutes":120,"time":"","say":"Awake for 2 hours"}"#;
        let expected = Understood {
            kind: Understanding::For,
            minutes: 120,
            time: String::new(),
            say: "Awake for 2 hours".to_owned(),
        };
        assert_eq!(
            understood(Provider::Anthropic, &claude(json)).unwrap(),
            expected
        );
        assert_eq!(understood(Provider::OpenAi, &chat(json)).unwrap(), expected);
        let fenced = format!("Here is the plan:\n```json\n{json}\n```");
        assert_eq!(
            understood(Provider::Custom, &chat(&fenced)).unwrap(),
            expected,
            "a service with no schema may say more than was asked"
        );
        let evening = chat(r#"{"kind":"until","minutes":0,"time":"18:30","say":"Until 6:30 pm"}"#);
        let evening = understood(Provider::OpenAi, &evening).unwrap();
        assert_eq!(evening.kind, Understanding::Until);
        assert_eq!(evening.time, "18:30");
    }

    #[test]
    fn half_an_answer_reads_as_unclear() {
        for text in [
            r#"{"kind":"for","minutes":0,"time":"","say":"Awake"}"#,
            r#"{"kind":"for","minutes":99999999,"time":"","say":"Awake forever"}"#,
            r#"{"kind":"until","minutes":0,"time":"half six","say":"Awake until half six"}"#,
            r#"{"kind":"until","minutes":0,"time":"25:00","say":"Awake"}"#,
        ] {
            let read = understood(Provider::Anthropic, &claude(text)).unwrap();
            assert_eq!(read.kind, Understanding::Unclear, "{text}");
            assert_eq!(read.minutes, 0);
            assert_eq!(read.time, "");
        }
        assert_eq!(
            understood(Provider::Anthropic, &claude("not json")),
            Err(AiError::Empty)
        );
        assert_eq!(
            understood(
                Provider::OpenAi,
                &chat(r#"{"kind":"dance","minutes":0,"time":"","say":""}"#)
            ),
            Err(AiError::Empty)
        );
    }

    #[test]
    fn a_status_becomes_something_a_person_can_act_on() {
        assert_eq!(AiError::from_status(401), AiError::BadKey);
        assert_eq!(AiError::from_status(404), AiError::UnknownModel);
        assert_eq!(AiError::from_status(429), AiError::Busy);
        assert_eq!(AiError::from_status(529), AiError::Busy);
        assert_eq!(
            AiError::from_status(500),
            AiError::Failed("HTTP 500".to_owned())
        );
        for error in [
            AiError::NoKey,
            AiError::Off,
            AiError::BadKey,
            AiError::Busy,
            AiError::NoModel,
            AiError::BadAddress,
            AiError::UnknownModel,
        ] {
            let sentence = error.sentence();
            assert!(!sentence.contains('!'));
            assert!(
                !sentence.contains("Claude"),
                "no one service is named: {sentence}"
            );
        }
    }

    #[test]
    fn understood_crosses_to_the_window_in_snake_case() {
        let read = Understood {
            kind: Understanding::Indefinite,
            minutes: 0,
            time: String::new(),
            say: "Awake until you say otherwise".to_owned(),
        };
        let json = serde_json::to_value(&read).unwrap();
        assert_eq!(json["kind"], "indefinite");
        assert_eq!(json["minutes"], 0);
    }
}
