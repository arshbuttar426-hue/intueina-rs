use candle_core::{Device, Tensor, DType};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use hf_hub::api::sync::Api;
use tokenizers::Tokenizer;

const QUERY_PREFIX: &str = "Represent this sentence for searching relevant passages: ";

pub struct RealEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl RealEmbedder {
    pub fn new(model_id: &str) -> anyhow::Result<Self> {
        let device = Device::Cpu;
        let api = Api::new()?;
        let repo = api.model(model_id.to_string());
        let config_path = repo.get("config.json")?;
        let weights_path = repo.get("model.safetensors")?;
        let tokenizer_path = repo.get("tokenizer.json")?;
        let config: Config = serde_json::from_str(&std::fs::read_to_string(config_path)?)?;
        let tokenizer = Tokenizer::from_file(&tokenizer_path).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, &device)? };
        let model = BertModel::load(vb, &config)?;
        Ok(Self { model, tokenizer, device })
    }

    fn embed_inner(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let tokens = self.tokenizer.encode(text, true).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let ids = tokens.get_ids();
        let attention = tokens.get_attention_mask();
        let seq_len = ids.len();
        let input_ids = Tensor::new(ids, &self.device)?.unsqueeze(0)?;
        let attention_mask = Tensor::new(attention, &self.device)?.unsqueeze(0)?;
        let token_type_ids = Tensor::zeros((1, seq_len), DType::U32, &self.device)?;
        let output = self.model.forward(&input_ids, &attention_mask, Some(&token_type_ids))?;
        let mask = attention_mask.to_dtype(DType::F32)?.unsqueeze(2)?;
        let summed = (output.broadcast_mul(&mask)?).sum(1)?;
        let mask_sum = mask.sum(1)?;
        let pooled = summed.broadcast_div(&mask_sum)?.squeeze(0)?;
        let mut vec: Vec<f32> = pooled.to_vec1()?;
        let n: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if n > 0.0 { for v in &mut vec { *v /= n; } }
        Ok(vec)
    }

    pub fn embed_query(&self, text: &str) -> anyhow::Result<super::EmbedQuery> {
        let v = self.embed_inner(&format!("{QUERY_PREFIX}{text}"))?;
        Ok(super::EmbedQuery(v))
    }
    pub fn embed_passage(&self, text: &str) -> anyhow::Result<super::EmbedPassage> {
        let v = self.embed_inner(text)?;
        Ok(super::EmbedPassage(v))
    }
}
