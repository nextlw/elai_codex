"""
Rotas HTTP customizadas para o servidor MCP Pipedrive.

Este módulo define endpoints HTTP adicionais além do protocolo MCP padrão,
como criação de sessões, health checks, etc.
"""

from typing import Optional
from starlette.requests import Request
from starlette.responses import JSONResponse, Response

from .session_manager import get_session_manager
from log_config import logger


async def create_session_endpoint(request: Request) -> Response:
    """
    Cria uma nova sessão de usuário.

    POST /sessions
    Body (opcional):
    {
        "user_id": "user123",
        "metadata": {"key": "value"}
    }

    Retorna:
    {
        "success": true,
        "session_id": "abc123...",
        "created_at": "2025-01-06T12:00:00",
        "expires_at": "2025-01-07T12:00:00"
    }
    """
    try:
        # Parse body (opcional)
        user_id: Optional[str] = None
        metadata: Optional[dict] = None

        try:
            body = await request.json()
            user_id = body.get("user_id")
            metadata = body.get("metadata")
        except Exception:
            # Body opcional, pode ser vazio
            pass

        # Cria a sessão
        session_manager = get_session_manager()
        session = session_manager.create_session(user_id=user_id, metadata=metadata)

        logger.info(f"Nova sessão criada: {session.session_id}")

        return JSONResponse({
            "success": True,
            "session_id": session.session_id,
            "created_at": session.created_at.isoformat(),
            "expires_at": session.expires_at.isoformat()
        })

    except Exception as e:
        logger.error(f"Erro ao criar sessão: {str(e)}")
        return JSONResponse(
            {"success": False, "error": str(e)},
            status_code=500
        )


async def get_session_endpoint(request: Request) -> Response:
    """
    Obtém informações de uma sessão existente.

    GET /sessions/{session_id}

    Retorna:
    {
        "success": true,
        "session": {
            "session_id": "abc123...",
            "created_at": "2025-01-06T12:00:00",
            "expires_at": "2025-01-07T12:00:00",
            "user_id": "user123",
            "metadata": {"key": "value"}
        }
    }
    """
    try:
        session_id = request.path_params.get("session_id")

        if not session_id:
            return JSONResponse(
                {"success": False, "error": "session_id é obrigatório"},
                status_code=400
            )

        session_manager = get_session_manager()
        session = session_manager.get_session(session_id)

        if session is None:
            return JSONResponse(
                {"success": False, "error": "Sessão não encontrada ou expirada"},
                status_code=404
            )

        return JSONResponse({
            "success": True,
            "session": session.to_dict()
        })

    except Exception as e:
        logger.error(f"Erro ao obter sessão: {str(e)}")
        return JSONResponse(
            {"success": False, "error": str(e)},
            status_code=500
        )


async def delete_session_endpoint(request: Request) -> Response:
    """
    Deleta uma sessão existente.

    DELETE /sessions/{session_id}

    Retorna:
    {
        "success": true,
        "message": "Sessão deletada com sucesso"
    }
    """
    try:
        session_id = request.path_params.get("session_id")

        if not session_id:
            return JSONResponse(
                {"success": False, "error": "session_id é obrigatório"},
                status_code=400
            )

        session_manager = get_session_manager()
        deleted = session_manager.delete_session(session_id)

        if not deleted:
            return JSONResponse(
                {"success": False, "error": "Sessão não encontrada"},
                status_code=404
            )

        logger.info(f"Sessão deletada: {session_id}")

        return JSONResponse({
            "success": True,
            "message": "Sessão deletada com sucesso"
        })

    except Exception as e:
        logger.error(f"Erro ao deletar sessão: {str(e)}")
        return JSONResponse(
            {"success": False, "error": str(e)},
            status_code=500
        )


async def health_check_endpoint(request: Request) -> Response:
    """
    Health check endpoint.

    GET /health

    Retorna:
    {
        "status": "ok",
        "active_sessions": 5
    }
    """
    session_manager = get_session_manager()
    session_count = session_manager.get_session_count()

    return JSONResponse({
        "status": "ok",
        "active_sessions": session_count
    })


def register_routes(mcp_instance):
    """
    Registra todas as rotas HTTP customizadas no servidor MCP.

    Args:
        mcp_instance: Instância do FastMCP
    """
    # Criar sessão
    mcp_instance.custom_route("/sessions", methods=["POST"])(create_session_endpoint)

    # Obter sessão
    mcp_instance.custom_route("/sessions/{session_id}", methods=["GET"])(get_session_endpoint)

    # Deletar sessão
    mcp_instance.custom_route("/sessions/{session_id}", methods=["DELETE"])(delete_session_endpoint)

    # Health check
    mcp_instance.custom_route("/health", methods=["GET"])(health_check_endpoint)

    logger.info("Rotas HTTP customizadas registradas")
