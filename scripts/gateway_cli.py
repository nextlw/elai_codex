#!/usr/bin/env python3
"""
Codex Gateway CLI - Production Client
Connects to Codex Gateway in Cloud Run (or local for development)
Supports: JSON-RPC, Exec Mode, WebSocket
"""

import asyncio
import json
import sys
import os
from typing import Optional, Dict, Any
import subprocess

try:
    import websockets
    import aiohttp
except ImportError:
    print("❌ Dependências não encontradas. Instalando...")
    subprocess.check_call([sys.executable, "-m", "pip", "install", "websockets", "aiohttp"])
    import websockets
    import aiohttp


class GatewayClient:
    def __init__(self, gateway_url: str, api_key: str):
        self.gateway_url = gateway_url.replace("https://", "wss://").replace("http://", "ws://")
        self.http_url = gateway_url.replace("wss://", "https://").replace("ws://", "http://")
        self.api_key = api_key
        self.session_id = f"cli-{os.getpid()}"
        self.message_id = 1

    async def health_check(self) -> bool:
        """Verifica se o gateway está saudável"""
        try:
            async with aiohttp.ClientSession() as session:
                async with session.get(f"{self.http_url}/health") as response:
                    data = await response.json()
                    return data.get("status") == "healthy"
        except Exception as e:
            print(f"❌ Erro ao verificar health: {e}")
            return False

    async def send_prompt_http(self, prompt: str) -> dict:
        """Envia prompt via HTTP JSON-RPC"""
        payload = {
            "jsonrpc": "2.0",
            "method": "conversation.prompt",
            "params": {
                "prompt": prompt,
                "session_id": self.session_id
            },
            "id": self.message_id
        }
        self.message_id += 1

        headers = {
            "X-API-Key": self.api_key,
            "Content-Type": "application/json"
        }

        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(
                    f"{self.http_url}/jsonrpc",
                    json=payload,
                    headers=headers
                ) as response:
                    try:
                        return await response.json()
                    except:
                        text = await response.text()
                        return {"error": f"HTTP {response.status}: {text}"}
        except Exception as e:
            return {"error": str(e)}

    async def exec_prompt(self, prompt: str) -> Dict[str, Any]:
        """Executa prompt via endpoint /exec (codex-exec integration)"""
        payload = {
            "prompt": prompt,
            "session_id": self.session_id
        }

        headers = {
            "X-API-Key": self.api_key,
            "Content-Type": "application/json"
        }

        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(
                    f"{self.http_url}/exec",
                    json=payload,
                    headers=headers
                ) as response:
                    try:
                        return await response.json()
                    except:
                        text = await response.text()
                        return {"error": f"HTTP {response.status}: {text}"}
        except Exception as e:
            return {"error": str(e)}

    async def websocket_session(self):
        """Sessão WebSocket interativa"""
        print("🌐 Conectando via WebSocket...")
        ws_url = f"{self.gateway_url}/ws?api_key={self.api_key}"

        try:
            async with websockets.connect(ws_url) as websocket:
                print("✅ Conectado!")
                print("Digite 'exit' para sair\n")

                async def receive_messages():
                    """Recebe mensagens do servidor"""
                    try:
                        async for message in websocket:
                            try:
                                data = json.loads(message)
                                print(f"\n📨 Servidor: {json.dumps(data, indent=2)}\n")
                            except:
                                print(f"\n📨 Servidor: {message}\n")
                    except Exception as e:
                        print(f"\n❌ Erro ao receber: {e}\n")

                async def send_messages():
                    """Envia mensagens do usuário"""
                    while True:
                        try:
                            user_input = await asyncio.get_event_loop().run_in_executor(
                                None, input, "💬 Você: "
                            )

                            if user_input.lower() == "exit":
                                break

                            await websocket.send(user_input)
                        except Exception as e:
                            print(f"\n❌ Erro ao enviar: {e}\n")
                            break

                # Executar recebimento e envio em paralelo
                await asyncio.gather(
                    receive_messages(),
                    send_messages()
                )

        except Exception as e:
            print(f"❌ Erro na conexão WebSocket: {e}")

    async def jsonrpc_interactive_mode(self):
        """Modo interativo via JSON-RPC HTTP"""
        print("🚀 Modo JSON-RPC HTTP")
        print(f"📡 Conectado a: {self.http_url}")
        print(f"🔑 Session ID: {self.session_id}")
        print("━" * 60)
        print("Digite seus prompts (ou 'exit' para sair, 'clear' para limpar)")
        print("━" * 60)
        print()

        while True:
            try:
                user_input = input("💬 Você: ").strip()

                if not user_input:
                    continue

                if user_input.lower() == "exit":
                    print("\n👋 Encerrando sessão...")
                    break

                if user_input.lower() == "clear":
                    os.system('clear' if os.name != 'nt' else 'cls')
                    continue

                print("⏳ Processando...")
                response = await self.send_prompt_http(user_input)

                if "error" in response:
                    print(f"❌ Erro: {response['error']}")
                elif "result" in response:
                    result = response["result"]
                    if isinstance(result, dict):
                        if "content" in result:
                            print(f"\n🤖 Resposta:\n{result['content']}\n")
                        else:
                            print(f"\n🤖 Resposta:")
                            print(json.dumps(result, indent=2))
                    else:
                        print(f"\n🤖 Resposta: {result}\n")
                else:
                    print(f"\n📦 Resposta completa:")
                    print(json.dumps(response, indent=2))

                print()

            except KeyboardInterrupt:
                print("\n\n👋 Interrompido pelo usuário. Encerrando...")
                break
            except EOFError:
                print("\n\n👋 EOF detectado. Encerrando...")
                break
            except Exception as e:
                print(f"\n❌ Erro inesperado: {e}")
                print()

    async def exec_interactive_mode(self):
        """Modo interativo via endpoint /exec (codex-exec integration)"""
        print("🚀 Modo EXEC (Codex-Exec Integration)")
        print(f"📡 Conectado a: {self.http_url}")
        print(f"🔑 Session ID: {self.session_id}")
        print("━" * 60)
        print("Digite seus prompts (ou 'exit' para sair)")
        print("Exemplo: create a python script that prints hello world")
        print("━" * 60)
        print()

        while True:
            try:
                user_input = input("💬 Você: ").strip()

                if not user_input:
                    continue

                if user_input.lower() == "exit":
                    print("\n👋 Encerrando sessão...")
                    break

                if user_input.lower() == "clear":
                    os.system('clear' if os.name != 'nt' else 'cls')
                    continue

                print("⏳ Executando...")
                response = await self.exec_prompt(user_input)

                if "error" in response:
                    print(f"❌ Erro: {response['error']}")
                elif "events" in response:
                    # Resposta JSONL com eventos
                    print(f"\n📦 Conversation ID: {response.get('conversation_id', 'N/A')}")
                    print(f"📊 Status: {response.get('status', 'N/A')}")
                    print(f"📝 Total Events: {len(response['events'])}\n")

                    # Mostrar eventos principais
                    for event in response['events']:
                        event_type = event.get('type', 'unknown')
                        if event_type == 'assistant_message':
                            content = event.get('content', '')
                            print(f"🤖 Assistant: {content}")
                        elif event_type == 'tool_use':
                            tool_name = event.get('tool_name', 'unknown')
                            print(f"🔧 Tool: {tool_name}")
                        elif event_type == 'error':
                            error_msg = event.get('message', 'Unknown error')
                            print(f"❌ Error: {error_msg}")
                else:
                    print(f"\n📦 Resposta:")
                    print(json.dumps(response, indent=2))

                print()

            except KeyboardInterrupt:
                print("\n\n👋 Interrompido pelo usuário. Encerrando...")
                break
            except EOFError:
                print("\n\n👋 EOF detectado. Encerrando...")
                break
            except Exception as e:
                print(f"\n❌ Erro inesperado: {e}")
                print()


def show_menu() -> str:
    """Exibe menu de opções e retorna escolha do usuário"""
    print("\n" + "━" * 60)
    print("🚀 CODEX GATEWAY CLI - PRODUCTION")
    print("━" * 60)
    print("\nEscolha o modo de operação:")
    print()
    print("  1. 📡 JSON-RPC HTTP - Prompts via HTTP")
    print("  2. ⚡ EXEC Mode - Codex-Exec Integration (recomendado)")
    print("  3. 🌐 WebSocket - Comunicação em tempo real")
    print("  4. 🏥 Health Check - Verificar status do gateway")
    print("  5. ❌ Sair")
    print()
    print("━" * 60)

    choice = input("Digite sua escolha (1-5): ").strip()
    return choice


def get_cloud_run_url(service_name: str = "wrapper", region: str = "us-central1") -> Optional[str]:
    """Obtém URL do Cloud Run via gcloud"""
    try:
        result = subprocess.run(
            [
                "gcloud", "run", "services", "describe", service_name,
                "--region", region,
                "--format=value(status.url)"
            ],
            capture_output=True,
            text=True,
            check=True,
            timeout=10
        )
        url = result.stdout.strip()
        if url:
            return url
    except Exception as e:
        print(f"⚠️  Não foi possível obter URL do Cloud Run: {e}")
    return None


def get_api_key() -> Optional[str]:
    """Obtém API key de múltiplas fontes (prioridade: env var > Secret Manager)"""
    # Prioridade 1: Variável de ambiente
    api_key = os.getenv("GATEWAY_KEY") or os.getenv("GATEWAY_API_KEY")
    if api_key:
        print("✅ API Key obtida de variável de ambiente")
        return api_key

    # Prioridade 2: Secret Manager (produção)
    try:
        result = subprocess.run(
            ["gcloud", "secrets", "versions", "access", "latest", "--secret=gateway-api-key"],
            capture_output=True,
            text=True,
            check=True,
            timeout=10
        )
        api_key = result.stdout.strip()
        if api_key:
            print("✅ API Key obtida do GCP Secret Manager")
            return api_key
    except subprocess.TimeoutExpired:
        print("⚠️  Timeout ao acessar Secret Manager")
    except subprocess.CalledProcessError as e:
        print(f"⚠️  Erro ao acessar Secret Manager: {e.stderr}")
    except FileNotFoundError:
        print("⚠️  gcloud CLI não encontrado (instale: https://cloud.google.com/sdk/install)")
    except Exception as e:
        print(f"⚠️  Erro inesperado ao obter secret: {e}")

    return None


def detect_environment() -> str:
    """Detecta se está em ambiente de desenvolvimento ou produção"""
    # Verifica se está rodando localmente
    if os.getenv("GATEWAY_URL", "").startswith("http://localhost"):
        return "development"

    # Verifica se gcloud está configurado e autenticado
    try:
        result = subprocess.run(
            ["gcloud", "config", "get-value", "project"],
            capture_output=True,
            text=True,
            check=True,
            timeout=5
        )
        if result.stdout.strip():
            return "production"
    except:
        pass

    return "development"


async def main():
    print("\n🔍 Detectando ambiente...")
    env = detect_environment()
    print(f"📍 Ambiente: {env.upper()}")

    # Obter configurações
    gateway_url = os.getenv("GATEWAY_URL")

    if not gateway_url:
        if env == "production":
            # Tentar obter URL do Cloud Run
            print("🔍 Obtendo URL do Cloud Run...")
            cloud_run_url = get_cloud_run_url()
            if cloud_run_url:
                gateway_url = cloud_run_url
                print(f"✅ Cloud Run URL: {gateway_url}")
            else:
                print("\n❌ Erro: Não foi possível obter URL do Cloud Run")
                print("💡 Configure manualmente: export GATEWAY_URL=https://your-service-url")
                sys.exit(1)
        else:
            # Desenvolvimento: usar localhost
            gateway_url = "http://localhost:3000"
            print(f"🔧 Usando URL de desenvolvimento: {gateway_url}")
    else:
        print(f"✅ Gateway URL configurada: {gateway_url}")

    # Obter API key
    print("\n🔑 Obtendo API key...")
    api_key = get_api_key()

    if not api_key:
        print("\n❌ Erro: API Key não encontrada")
        print("\n💡 Opções para configurar API key:")
        print("  1. Variável de ambiente:")
        print("     export GATEWAY_KEY=sua-api-key")
        print("  2. GCP Secret Manager (produção):")
        print("     gcloud secrets versions access latest --secret=gateway-api-key")
        print("\n📚 Consulte a documentação para mais informações")
        sys.exit(1)

    # Iniciar cliente
    print(f"\n🚀 Iniciando cliente do Codex Gateway...")
    print(f"📡 URL: {gateway_url}")
    print(f"🌍 Ambiente: {env}")

    client = GatewayClient(gateway_url, api_key)

    # Verificar conectividade
    print("\n🏥 Verificando conectividade...")
    is_healthy = await client.health_check()
    if not is_healthy:
        print("⚠️  Gateway não está respondendo ao health check")
        print("   Continuando mesmo assim...")
    else:
        print("✅ Gateway está saudável!")

    # Loop do menu
    while True:
        choice = show_menu()

        if choice == "1":
            # JSON-RPC HTTP Mode
            await client.jsonrpc_interactive_mode()

        elif choice == "2":
            # EXEC Mode
            await client.exec_interactive_mode()

        elif choice == "3":
            # WebSocket Mode
            await client.websocket_session()

        elif choice == "4":
            # Health Check
            print("\n🏥 Verificando status do gateway...")
            is_healthy = await client.health_check()
            if is_healthy:
                print("✅ Gateway está saudável!")
            else:
                print("❌ Gateway não está respondendo corretamente")

        elif choice == "5":
            # Sair
            print("\n👋 Encerrando...")
            break

        else:
            print(f"\n❌ Opção inválida: {choice}")
            print("Por favor, escolha uma opção de 1 a 5")


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n👋 Até logo!")
        sys.exit(0)
    except Exception as e:
        print(f"\n❌ Erro fatal: {e}")
        sys.exit(1)
