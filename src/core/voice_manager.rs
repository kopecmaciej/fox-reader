use crate::core::kokoros_manager::KokorosTTS;
use rodio::buffer::SamplesBuffer;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::OnceCell;

static KOKOROS_TTS: OnceCell<Arc<KokorosTTS>> = OnceCell::const_new();

pub struct VoiceManager {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Language {
    pub code: String,
    pub name_english: String,
    pub region: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct File {
    size_bytes: u64,
    md5_digest: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Voice {
    pub name: String,
    pub key: String,
    pub language: Language,
    pub quality: String,
    pub traits: String,
    pub is_default: Option<bool>,
}

impl VoiceManager {
    pub async fn init_kokoros() -> Result<(), Box<dyn Error + Send + Sync>> {
        let kokoros = KokorosTTS::new().await?;
        KOKOROS_TTS
            .set(Arc::new(kokoros))
            .map_err(|_| "Failed to initialize Kokoros TTS")?;
        Ok(())
    }

    pub async fn generate_kokoros_speech(
        text: &str,
        voice_style: &str,
        speed: f32,
    ) -> Result<SamplesBuffer<f32>, Box<dyn Error + Send + Sync>> {
        let kokoros = KOKOROS_TTS.get().ok_or("Kokoros TTS not initialized")?;

        kokoros.generate_speech(text, voice_style, speed).await
    }

    pub async fn save_kokoros_speech_to_file(
        text: &str,
        voice_style: &str,
        speed: f32,
        output_path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let kokoros = KOKOROS_TTS.get().ok_or("Kokoros TTS not initialized")?;

        kokoros
            .save_speech_to_file(text, voice_style, speed, output_path)
            .await
    }

    pub fn get_kokoros_voices() -> Vec<String> {
        KokorosTTS::get_available_voices()
    }

    pub fn get_kokoros_voice_rows() -> Vec<Voice> {
        let mut kokoros_voices = KokorosTTS::get_available_voices();

        kokoros_voices.sort_by(|a, b| {
            let (_, _, country_a, _) = Self::get_language_info_from_voice_style(a);
            let (_, _, country_b, _) = Self::get_language_info_from_voice_style(b);

            country_a.cmp(&country_b).then_with(|| a.cmp(b))
        });

        kokoros_voices
            .into_iter()
            .map(|voice_style| {
                let (language_code, language_name, region, _flag) =
                    Self::get_language_info_from_voice_style(&voice_style);
                let friendly_name = Self::get_friendly_voice_name(&voice_style);
                let quality_grade = Self::get_voice_quality_grade(&voice_style);
                let traits = Self::get_voice_traits(&voice_style);

                Voice {
                    name: friendly_name,
                    key: voice_style.to_string(),
                    language: Language {
                        code: language_code,
                        name_english: language_name,
                        region,
                    },
                    quality: quality_grade,
                    traits,
                    is_default: Some(voice_style == "af_heart"),
                }
            })
            .collect()
    }

    pub async fn list_all_available_voices_with_kokoros(
    ) -> Result<BTreeMap<String, Voice>, Box<dyn Error>> {
        let mut all_voices = BTreeMap::new();

        let kokoros_voices = Self::get_kokoros_voice_rows();
        for voice in kokoros_voices {
            all_voices.insert(voice.key.clone(), voice);
        }

        Ok(all_voices)
    }

    fn get_language_info_from_voice_style(voice_style: &str) -> (String, String, String, String) {
        let prefix = voice_style.get(0..2).unwrap_or("");
        match prefix {
            // American English
            "af" | "am" => (
                "en-US".to_string(),
                "English".to_string(),
                "United States".to_string(),
                "🇺🇸".to_string(),
            ),
            // British English
            "bf" | "bm" => (
                "en-GB".to_string(),
                "English".to_string(),
                "United Kingdom".to_string(),
                "🇬🇧".to_string(),
            ),
            // Japanese
            "jf" | "jm" => (
                "ja".to_string(),
                "Japanese".to_string(),
                "Japan".to_string(),
                "🇯🇵".to_string(),
            ),
            // Mandarin Chinese
            "zf" | "zm" => (
                "zh-CN".to_string(),
                "Chinese".to_string(),
                "China".to_string(),
                "🇨🇳".to_string(),
            ),
            // Spanish
            "ef" | "em" => (
                "es".to_string(),
                "Spanish".to_string(),
                "Spain".to_string(),
                "🇪🇸".to_string(),
            ),
            // French
            "ff" | "fm" => (
                "fr".to_string(),
                "French".to_string(),
                "France".to_string(),
                "🇫🇷".to_string(),
            ),
            // Hindi
            "hf" | "hm" => (
                "hi".to_string(),
                "Hindi".to_string(),
                "India".to_string(),
                "🇮🇳".to_string(),
            ),
            // Italian
            "if" | "im" => (
                "it".to_string(),
                "Italian".to_string(),
                "Italy".to_string(),
                "🇮🇹".to_string(),
            ),
            // Brazilian Portuguese
            "pf" | "pm" => (
                "pt-BR".to_string(),
                "Portuguese".to_string(),
                "Brazil".to_string(),
                "🇧🇷".to_string(),
            ),
            // Default to American English for unknown prefixes
            _ => (
                "en-US".to_string(),
                "English".to_string(),
                "United States".to_string(),
                "🇺🇸".to_string(),
            ),
        }
    }

    pub fn get_voice_traits(voice_style: &str) -> String {
        let base_gender_trait = match voice_style.get(1..2).unwrap_or("") {
            "f" => "♀️", // Female
            "m" => "♂️", // Male
            _ => "",
        };

        let special_traits = match voice_style {
            // American English special traits
            "af_heart" => "❤️",            // Heart
            "af_bella" => "🔥",            // Fire
            "af_nicole" => "🎧",           // Headphones
            "af_sky" | "am_santa" => "🤏", // Short training

            // Japanese special traits
            "jf_nezumi" | "jm_kumo" => "🤏", // Short training

            _ => "",
        };

        if special_traits.is_empty() {
            base_gender_trait.to_string()
        } else {
            format!("{}{}", base_gender_trait, special_traits)
        }
    }

    fn get_friendly_voice_name(voice_style: &str) -> String {
        let (_, _, country, flag) = Self::get_language_info_from_voice_style(voice_style);

        let name_part = voice_style.get(3..).unwrap_or(voice_style);
        let formatted_name = name_part
            .split('_')
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        format!("{} {} - {}", flag, formatted_name, country)
    }

    fn get_voice_quality_grade(voice_style: &str) -> String {
        match voice_style {
            // American English
            "af_heart" => "A".to_string(),
            "af_alloy" => "C".to_string(),
            "af_aoede" => "C+".to_string(),
            "af_bella" => "A-".to_string(),
            "af_jessica" => "D".to_string(),
            "af_kore" => "C+".to_string(),
            "af_nicole" => "B-".to_string(),
            "af_nova" => "C".to_string(),
            "af_river" => "D".to_string(),
            "af_sarah" => "C+".to_string(),
            "af_sky" => "C-".to_string(),
            "am_adam" => "F+".to_string(),
            "am_echo" => "D".to_string(),
            "am_eric" => "D".to_string(),
            "am_fenrir" => "C+".to_string(),
            "am_liam" => "D".to_string(),
            "am_michael" => "C+".to_string(),
            "am_onyx" => "D".to_string(),
            "am_puck" => "C+".to_string(),
            "am_santa" => "D-".to_string(),

            // British English
            "bf_alice" => "D".to_string(),
            "bf_emma" => "B-".to_string(),
            "bf_isabella" => "C".to_string(),
            "bf_lily" => "D".to_string(),
            "bm_daniel" => "D".to_string(),
            "bm_fable" => "C".to_string(),
            "bm_george" => "C".to_string(),
            "bm_lewis" => "D+".to_string(),

            // Japanese
            "jf_alpha" => "C+".to_string(),
            "jf_gongitsune" => "C".to_string(),
            "jf_nezumi" => "C-".to_string(),
            "jf_tebukuro" => "C".to_string(),
            "jm_kumo" => "C-".to_string(),

            // Mandarin Chinese
            "zf_xiaobei" => "D".to_string(),
            "zf_xiaoni" => "D".to_string(),
            "zf_xiaoxiao" => "D".to_string(),
            "zf_xiaoyi" => "D".to_string(),
            "zm_yunjian" => "D".to_string(),
            "zm_yunxi" => "D".to_string(),
            "zm_yunxia" => "D".to_string(),
            "zm_yunyang" => "D".to_string(),

            // Spanish (no specific grades in table, using reasonable defaults)
            "ef_dora" => "C".to_string(),
            "em_alex" => "C".to_string(),
            "em_santa" => "C".to_string(),

            // French
            "ff_siwis" => "B-".to_string(),

            // Hindi
            "hf_alpha" => "C".to_string(),
            "hf_beta" => "C".to_string(),
            "hm_omega" => "C".to_string(),
            "hm_psi" => "C".to_string(),

            // Italian
            "if_sara" => "C".to_string(),
            "im_nicola" => "C".to_string(),

            // Brazilian Portuguese (no specific grades in table, using reasonable defaults)
            "pf_dora" => "C".to_string(),
            "pm_alex" => "C".to_string(),
            "pm_santa" => "C".to_string(),

            // Default fallback
            _ => "C".to_string(),
        }
    }
}
