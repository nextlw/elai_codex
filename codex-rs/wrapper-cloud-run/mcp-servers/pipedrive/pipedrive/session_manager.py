"""
Gerenciador de sessões para o servidor MCP Pipedrive.

Este módulo fornece funcionalidade para criar e gerenciar sessões de usuário.
Por enquanto, usa armazenamento em memória, mas pode ser facilmente estendido
para usar Redis ou Firestore.
"""

import uuid
from datetime import datetime, timedelta
from typing import Dict, Optional
from dataclasses import dataclass


@dataclass
class Session:
    """Representa uma sessão de usuário."""

    session_id: str
    created_at: datetime
    expires_at: datetime
    user_id: Optional[str] = None
    metadata: Dict = None

    def __post_init__(self):
        if self.metadata is None:
            self.metadata = {}

    def is_expired(self) -> bool:
        """Verifica se a sessão expirou."""
        return datetime.now() > self.expires_at

    def to_dict(self) -> Dict:
        """Converte a sessão para um dicionário."""
        return {
            "session_id": self.session_id,
            "created_at": self.created_at.isoformat(),
            "expires_at": self.expires_at.isoformat(),
            "user_id": self.user_id,
            "metadata": self.metadata
        }


class SessionManager:
    """Gerenciador de sessões em memória."""

    def __init__(self, default_ttl_hours: int = 24):
        """
        Inicializa o gerenciador de sessões.

        Args:
            default_ttl_hours: Tempo de vida padrão das sessões em horas
        """
        self._sessions: Dict[str, Session] = {}
        self._default_ttl = timedelta(hours=default_ttl_hours)

    def create_session(self, user_id: Optional[str] = None, metadata: Optional[Dict] = None) -> Session:
        """
        Cria uma nova sessão.

        Args:
            user_id: ID do usuário associado à sessão (opcional)
            metadata: Metadados adicionais da sessão (opcional)

        Returns:
            Session: A sessão criada
        """
        session_id = str(uuid.uuid4()).replace('-', '')
        now = datetime.now()
        expires_at = now + self._default_ttl

        session = Session(
            session_id=session_id,
            created_at=now,
            expires_at=expires_at,
            user_id=user_id,
            metadata=metadata or {}
        )

        self._sessions[session_id] = session
        return session

    def get_session(self, session_id: str) -> Optional[Session]:
        """
        Obtém uma sessão pelo ID.

        Args:
            session_id: ID da sessão

        Returns:
            Optional[Session]: A sessão se encontrada e não expirada, None caso contrário
        """
        session = self._sessions.get(session_id)

        if session is None:
            return None

        if session.is_expired():
            self.delete_session(session_id)
            return None

        return session

    def update_session(self, session_id: str, user_id: Optional[str] = None,
                      metadata: Optional[Dict] = None) -> Optional[Session]:
        """
        Atualiza uma sessão existente.

        Args:
            session_id: ID da sessão
            user_id: Novo ID do usuário (opcional)
            metadata: Novos metadados (opcional)

        Returns:
            Optional[Session]: A sessão atualizada ou None se não encontrada
        """
        session = self.get_session(session_id)

        if session is None:
            return None

        if user_id is not None:
            session.user_id = user_id

        if metadata is not None:
            session.metadata.update(metadata)

        return session

    def delete_session(self, session_id: str) -> bool:
        """
        Deleta uma sessão.

        Args:
            session_id: ID da sessão

        Returns:
            bool: True se a sessão foi deletada, False se não encontrada
        """
        if session_id in self._sessions:
            del self._sessions[session_id]
            return True
        return False

    def cleanup_expired_sessions(self) -> int:
        """
        Remove todas as sessões expiradas.

        Returns:
            int: Número de sessões removidas
        """
        expired_ids = [
            session_id
            for session_id, session in self._sessions.items()
            if session.is_expired()
        ]

        for session_id in expired_ids:
            del self._sessions[session_id]

        return len(expired_ids)

    def get_session_count(self) -> int:
        """Retorna o número de sessões ativas."""
        return len(self._sessions)


# Singleton instance
_session_manager: Optional[SessionManager] = None


def get_session_manager() -> SessionManager:
    """Retorna a instância singleton do SessionManager."""
    global _session_manager
    if _session_manager is None:
        _session_manager = SessionManager()
    return _session_manager
