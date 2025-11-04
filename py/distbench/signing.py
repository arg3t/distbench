"""Cryptographic message signing using Ed25519.

This module provides support for cryptographically signed messages,
which is essential for Byzantine fault-tolerant algorithms.
"""

import hashlib
from dataclasses import asdict, dataclass, is_dataclass
from typing import Any, Generic, TypeVar

from nacl.encoding import RawEncoder
from nacl.signing import SigningKey, VerifyKey

from distbench.community import PeerId

M = TypeVar("M")


@dataclass
class Signed(Generic[M]):
    """Wrapper for cryptographically signed messages.

    This class wraps a message with its Ed25519 signature and signer identity.
    It provides transparent access to the inner message through __getattr__.
    """

    value: M
    signature: bytes
    signer: PeerId

    def __str__(self) -> str:
        """Display the message with signer information.

        Returns:
            String showing the value and who signed it
        """
        return f"{self.value} (signed by {self.signer})"

    def __repr__(self) -> str:
        """Return a representation of the signed message.

        Returns:
            String in format: <value> (signed by <node>)
        """
        return f"{self.value} (signed by {self.signer})"

    def __getattr__(self, name: str) -> Any:
        """Provide transparent access to inner value's attributes.

        This implements a "deref" pattern where accessing attributes on the
        Signed wrapper delegates to the wrapped value.

        Args:
            name: Attribute name to access

        Returns:
            Attribute value from the inner message
        """
        # Avoid infinite recursion for dataclass fields
        if name in ["value", "signature", "signer"]:
            return object.__getattribute__(self, name)
        return getattr(object.__getattribute__(self, "value"), name)

    def inner(self) -> M:
        """Get the inner wrapped value.

        Returns:
            The original message without the signature wrapper
        """
        return self.value


class KeyPair:
    """Ed25519 key pair for signing and verifying messages.

    Each node generates a unique key pair on startup for creating
    and verifying cryptographic signatures.
    """

    def __init__(self, peer_id: PeerId) -> None:
        """Initialize a new key pair.

        Args:
            peer_id: The peer ID of the node that owns this key pair
        """
        self.peer_id = peer_id
        self.signing_key = SigningKey.generate()
        self.verify_key = self.signing_key.verify_key

    def public_key_bytes(self) -> bytes:
        """Get the public key as raw bytes.

        Returns:
            32-byte Ed25519 public key
        """
        return bytes(self.verify_key)

    def sign(self, message: M) -> Signed[M]:
        """Create a cryptographically signed message.

        Args:
            message: Message to sign (any object)

        Returns:
            Signed wrapper containing the message, signature, and signer ID
        """
        # Compute digest of the message
        digest = self._digest(message)

        # Sign the digest
        signature = self.signing_key.sign(digest, encoder=RawEncoder).signature

        # Return signed wrapper
        return Signed(value=message, signature=signature, signer=self.peer_id)

    def verify(self, signed: Signed[M], verify_key: VerifyKey) -> bool:
        """Verify a signed message against a public key.

        Args:
            signed: Signed message to verify
            verify_key: Public key to verify against

        Returns:
            True if the signature is valid, False otherwise
        """
        digest = self._digest(signed.value)
        try:
            verify_key.verify(digest, signed.signature, encoder=RawEncoder)
            return True
        except Exception:
            return False

    @staticmethod
    def verify_key_from_bytes(key_bytes: bytes) -> VerifyKey:
        """Create a VerifyKey from raw bytes.

        Args:
            key_bytes: 32-byte Ed25519 public key

        Returns:
            VerifyKey instance for signature verification
        """
        return VerifyKey(key_bytes)

    def _digest(self, obj: Any) -> bytes:
        """Compute a cryptographic digest of an object.

        Args:
            obj: Object to hash

        Returns:
            32-byte BLAKE2b digest
        """
        # Convert object to bytes for hashing
        if is_dataclass(obj):
            # Use dataclass fields in a canonical order
            data = asdict(obj)
            # Sort keys for deterministic ordering
            canonical = str(sorted(data.items()))
        elif isinstance(obj, (str, int, float, bool)):
            canonical = str(obj)
        elif isinstance(obj, bytes):
            return hashlib.blake2b(obj, digest_size=32).digest()
        else:
            # Fallback to repr
            canonical = repr(obj)

        return hashlib.blake2b(canonical.encode("utf-8"), digest_size=32).digest()
