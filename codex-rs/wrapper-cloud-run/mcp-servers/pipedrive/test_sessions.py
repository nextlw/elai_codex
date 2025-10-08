#!/usr/bin/env python3
"""
Script de teste para os endpoints de sessão.
"""

import asyncio
import httpx


async def test_session_endpoints():
    """Testa os endpoints de sessão."""
    base_url = "http://localhost:3000"

    async with httpx.AsyncClient() as client:
        print("1. Testando criação de sessão...")
        response = await client.post(f"{base_url}/sessions", json={
            "user_id": "user123",
            "metadata": {"test": "value"}
        })
        print(f"Status: {response.status_code}")
        result = response.json()
        print(f"Resposta: {result}")

        if result.get("success"):
            session_id = result.get("session_id")
            print(f"\n2. Testando obtenção de sessão {session_id}...")
            response = await client.get(f"{base_url}/sessions/{session_id}")
            print(f"Status: {response.status_code}")
            print(f"Resposta: {response.json()}")

            print(f"\n3. Testando health check...")
            response = await client.get(f"{base_url}/health")
            print(f"Status: {response.status_code}")
            print(f"Resposta: {response.json()}")

            print(f"\n4. Testando deleção de sessão {session_id}...")
            response = await client.delete(f"{base_url}/sessions/{session_id}")
            print(f"Status: {response.status_code}")
            print(f"Resposta: {response.json()}")

            print(f"\n5. Verificando se sessão foi deletada...")
            response = await client.get(f"{base_url}/sessions/{session_id}")
            print(f"Status: {response.status_code}")
            print(f"Resposta: {response.json()}")


if __name__ == "__main__":
    print("Testando endpoints de sessão...")
    print("=" * 50)
    asyncio.run(test_session_endpoints())
