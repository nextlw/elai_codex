#!/usr/bin/env node

/**
 * Proxy de Autenticação para Pipedrive MCP no Cloud Run
 *
 * Este proxy local injeta automaticamente o token de autenticação GCP
 * permitindo que o MCP client conecte ao Cloud Run sem configurar credenciais.
 */

const { spawn } = require('child_process');
const { GoogleAuth } = require('google-auth-library');

const CLOUD_RUN_URL = 'https://pipedrive-mcp-467992722695.us-central1.run.app/sse';
const SERVICE_ACCOUNT = 'codex-wrapper-sa@elaihub-prod.iam.gserviceaccount.com';

async function getAuthToken() {
  try {
    const auth = new GoogleAuth({
      scopes: ['https://www.googleapis.com/auth/cloud-platform'],
    });

    const client = await auth.getClient();
    const tokenResponse = await client.getAccessToken();

    return tokenResponse.token;
  } catch (error) {
    console.error('Erro ao obter token:', error.message);
    throw error;
  }
}

async function startProxy() {
  try {
    const token = await getAuthToken();

    // Inicia mcp-remote com header de autenticação
    const proxy = spawn('npx', [
      '-y',
      'mcp-remote@latest',
      CLOUD_RUN_URL,
      '--header',
      `Authorization: Bearer ${token}`
    ], {
      stdio: 'inherit',
      env: {
        ...process.env,
      }
    });

    proxy.on('error', (error) => {
      console.error('Erro no proxy:', error);
      process.exit(1);
    });

    proxy.on('exit', (code) => {
      process.exit(code || 0);
    });

    // Renovar token a cada 50 minutos (tokens expiram em 1h)
    setInterval(async () => {
      try {
        await getAuthToken();
        console.log('Token renovado');
      } catch (error) {
        console.error('Erro ao renovar token:', error.message);
      }
    }, 50 * 60 * 1000);

  } catch (error) {
    console.error('Falha ao iniciar proxy:', error.message);
    process.exit(1);
  }
}

startProxy();
