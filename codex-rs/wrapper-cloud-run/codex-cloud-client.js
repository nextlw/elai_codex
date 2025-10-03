#!/usr/bin/env node
/**
 * Cliente JavaScript para Codex Cloud Wrapper
 * Autor: Nexcode Team
 * Data: 2025-10-03
 */

const { spawn } = require('child_process');
const https = require('https');

class CodexCloudClient {
  constructor(baseUrl = 'https://codex-wrapper-467992722695.us-central1.run.app') {
    this.baseUrl = baseUrl;
    this.token = null;
  }

  /**
   * Obtém token de autenticação do gcloud
   */
  async getToken() {
    if (this.token) return this.token;

    return new Promise((resolve, reject) => {
      const gcloud = spawn('gcloud', ['auth', 'print-identity-token']);
      let token = '';

      gcloud.stdout.on('data', (data) => {
        token += data.toString();
      });

      gcloud.stderr.on('data', (data) => {
        console.error(`Erro gcloud: ${data}`);
      });

      gcloud.on('close', (code) => {
        if (code !== 0) {
          reject(new Error('Falha ao obter token. Execute: gcloud auth login adm@nexcode.live'));
        } else {
          this.token = token.trim();
          resolve(this.token);
        }
      });
    });
  }

  /**
   * Faz requisição HTTP
   */
  async _makeRequest(path, payload) {
    const token = await this.getToken();
    const url = new URL(path, this.baseUrl);

    return new Promise((resolve, reject) => {
      const postData = JSON.stringify(payload);

      const options = {
        hostname: url.hostname,
        port: 443,
        path: url.pathname,
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Content-Length': Buffer.byteLength(postData),
          'Authorization': `Bearer ${token}`
        }
      };

      const req = https.request(options, (res) => {
        if (res.statusCode !== 200) {
          reject(new Error(`HTTP ${res.statusCode}: ${res.statusMessage}`));
          return;
        }

        resolve(res);
      });

      req.on('error', reject);
      req.write(postData);
      req.end();
    });
  }

  /**
   * Executa prompt e retorna stream de eventos
   */
  async execStream(prompt, options = {}) {
    const {
      model = 'gpt-4o-mini',
      timeout_ms = 60000,
      session_id = null,
      onEvent = null
    } = options;

    const payload = {
      prompt,
      model,
      timeout_ms
    };
    if (session_id) payload.session_id = session_id;

    const response = await this._makeRequest('/api/v1/exec/stream', payload);

    // Parse SSE stream
    const events = [];
    let currentEvent = null;
    let buffer = '';

    return new Promise((resolve, reject) => {
      response.on('data', (chunk) => {
        buffer += chunk.toString();
        const lines = buffer.split('\n');
        buffer = lines.pop() || ''; // Mantém última linha incompleta

        for (const line of lines) {
          if (line.startsWith('event:')) {
            currentEvent = line.substring(6).trim();
          } else if (line.startsWith('data:')) {
            const data = line.substring(5).trim();
            let parsedData;

            try {
              parsedData = JSON.parse(data);
            } catch {
              parsedData = data;
            }

            const event = {
              event: currentEvent || 'unknown',
              data: parsedData
            };

            events.push(event);

            if (onEvent) {
              onEvent(event);
            }
          }
        }
      });

      response.on('end', () => {
        resolve(events);
      });

      response.on('error', reject);
    });
  }

  /**
   * Executa prompt e retorna apenas resposta final
   */
  async execSimple(prompt, options = {}) {
    const { verbose = false, model = 'gpt-4o-mini' } = options;
    let finalMessage = '';

    await this.execStream(prompt, {
      model,
      onEvent: (event) => {
        if (verbose) {
          console.log(`[${event.event}]`, JSON.stringify(event.data));
        }

        // Captura mensagem final
        if (['agent_message', 'agent_output'].includes(event.event)) {
          if (event.data?.message) {
            finalMessage = event.data.message;
          }
        } else if (event.event === 'task_complete') {
          if (event.data?.last_agent_message) {
            finalMessage = event.data.last_agent_message;
          }
        }
      }
    });

    return finalMessage;
  }

  /**
   * Stream em tempo real mostrando deltas
   */
  async execLive(prompt, model = 'gpt-4o-mini') {
    await this.execStream(prompt, {
      model,
      onEvent: (event) => {
        if (event.event === 'agent_message_delta') {
          process.stdout.write(event.data?.delta || '');
        } else if (event.event === 'task_complete') {
          console.log('\n[Tarefa concluída]');
        } else if (event.event === 'error') {
          console.error('\n[Erro]', event.data);
        }
      }
    });
  }
}

// Exemplos de uso
async function main() {
  const client = new CodexCloudClient();

  console.log('=== Exemplo 1: Pergunta Simples ===');
  const resposta1 = await client.execSimple('What is 2+2? Answer with just the number.');
  console.log(`Resposta: ${resposta1}\n`);

  console.log('=== Exemplo 2: Criar e Executar Script ===');
  const resposta2 = await client.execSimple(
    "Create a Python script that prints 'Hello from Node.js Client!' and execute it",
    { verbose: true }
  );
  console.log(`\nResposta final: ${resposta2}\n`);

  console.log('=== Exemplo 3: Stream ao Vivo ===');
  await client.execLive('Calculate 5 + 7 and explain the result');
}

// Executar se chamado diretamente
if (require.main === module) {
  main().catch(console.error);
}

module.exports = CodexCloudClient;
