#!/usr/bin/env python3
"""
Cliente Python para Codex Cloud Wrapper
Autor: Nexcode Team
Data: 2025-10-03
"""

import json
import subprocess
import sys
import time
from typing import Optional, Dict, Any, Generator
import requests

class CodexCloudClient:
    """Cliente para interagir com Codex Cloud Wrapper"""

    def __init__(self, base_url: str = "https://codex-wrapper-467992722695.us-central1.run.app"):
        self.base_url = base_url
        self.token = self._get_token()

    def _get_token(self) -> str:
        """Obtém token de autenticação do gcloud"""
        try:
            result = subprocess.run(
                ['gcloud', 'auth', 'print-identity-token'],
                capture_output=True,
                text=True,
                check=True
            )
            return result.stdout.strip()
        except subprocess.CalledProcessError as e:
            print(f"Erro ao obter token: {e}", file=sys.stderr)
            print("Execute: gcloud auth login adm@nexcode.live", file=sys.stderr)
            sys.exit(1)

    def exec_stream(
        self,
        prompt: str,
        model: str = "gpt-4o-mini",
        timeout_ms: int = 60000,
        session_id: Optional[str] = None
    ) -> Generator[Dict[str, Any], None, None]:
        """
        Executa prompt e retorna stream de eventos

        Args:
            prompt: Instrução para o Codex
            model: Modelo a usar (gpt-4o-mini, gpt-4o, claude-sonnet-4, etc)
            timeout_ms: Timeout em milissegundos
            session_id: ID da sessão (opcional, será gerado se não fornecido)

        Yields:
            Dicionários com eventos SSE
        """
        url = f"{self.base_url}/api/v1/exec/stream"
        headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {self.token}"
        }
        payload = {
            "prompt": prompt,
            "model": model,
            "timeout_ms": timeout_ms
        }
        if session_id:
            payload["session_id"] = session_id

        try:
            with requests.post(url, json=payload, headers=headers, stream=True) as response:
                response.raise_for_status()

                event_type = None
                # Parse SSE stream
                for line in response.iter_lines(decode_unicode=True):
                    if not line:
                        continue

                    if line.startswith('event:'):
                        event_type = line[6:].strip()
                    elif line.startswith('data:'):
                        data = line[5:].strip()
                        try:
                            data_json = json.loads(data)
                            yield {
                                'event': event_type if event_type else 'unknown',
                                'data': data_json
                            }
                        except json.JSONDecodeError:
                            yield {
                                'event': event_type if event_type else 'unknown',
                                'data': data
                            }
        except requests.exceptions.RequestException as e:
            print(f"Erro na requisição: {e}", file=sys.stderr)
            raise

    def exec_simple(
        self,
        prompt: str,
        model: str = "gpt-4o-mini",
        verbose: bool = False
    ) -> str:
        """
        Executa prompt e retorna apenas a resposta final

        Args:
            prompt: Instrução para o Codex
            model: Modelo a usar
            verbose: Se True, mostra eventos intermediários

        Returns:
            Resposta final do agente
        """
        final_message = ""

        for event in self.exec_stream(prompt, model):
            if verbose:
                print(f"[{event['event']}] {event['data']}")

            # Captura mensagem final
            if event['event'] in ['agent_message', 'agent_output']:
                if isinstance(event['data'], dict) and 'message' in event['data']:
                    final_message = event['data']['message']
            elif event['event'] == 'task_complete':
                if isinstance(event['data'], dict) and 'last_agent_message' in event['data']:
                    if event['data']['last_agent_message']:
                        final_message = event['data']['last_agent_message']

        return final_message


def main():
    """Exemplos de uso"""
    client = CodexCloudClient()

    print("=== Exemplo 1: Pergunta Simples ===")
    resposta = client.exec_simple("What is 2+2? Answer with just the number.")
    print(f"Resposta: {resposta}\n")

    print("=== Exemplo 2: Criar e Executar Script ===")
    resposta = client.exec_simple(
        "Create a Python script that prints 'Hello from Cloud!' and execute it",
        verbose=True
    )
    print(f"\nResposta final: {resposta}\n")

    print("=== Exemplo 3: Stream com Eventos ===")
    for event in client.exec_stream("Calculate 5 + 7 and explain the result"):
        if event['event'] == 'agent_message_delta':
            # Imprime deltas em tempo real
            print(event['data'].get('delta', ''), end='', flush=True)
        elif event['event'] == 'task_complete':
            print("\n[Tarefa concluída]")
            break


if __name__ == "__main__":
    main()
