<?php

namespace Nativella\Nfc;

use Nativella\Exception\UnexpectedResponseException;

/**
 * A record of a tag's NDEF message. $type, $id and $payload are bytes, and
 * text() and uri() read the records most tags hold.
 */
final readonly class Record
{
    /** The index is the code a URI record holds. */
    private const array URI_PREFIXES = [
        '', 'http://www.', 'https://www.', 'http://', 'https://', 'tel:', 'mailto:', 'ftp://anonymous:anonymous@', 'ftp://ftp.',
        'ftps://', 'sftp://', 'smb://', 'nfs://', 'ftp://', 'dav://', 'news:', 'telnet://', 'imap:', 'rtsp://', 'urn:', 'pop:',
        'sip:', 'sips:', 'tftp:', 'btspp://', 'btl2cap://', 'btgoep://', 'tcpobex://', 'irdaobex://', 'file://', 'urn:epc:id:',
        'urn:epc:tag:', 'urn:epc:pat:', 'urn:epc:raw:', 'urn:epc:', 'urn:nfc:',
    ];

    public function __construct(
        public TypeNameFormat $format,
        public string $type,
        public string $id,
        public string $payload,
    ) {
    }

    /**
     * @param array{tnf: int, type: string, id: string, payload: string} $data the bytes in base64
     */
    public static function fromArray(array $data): self
    {
        return new self(
            TypeNameFormat::tryFrom($data['tnf']) ?? TypeNameFormat::Unknown,
            self::bytes($data['type']),
            self::bytes($data['id']),
            self::bytes($data['payload']),
        );
    }

    /**
     * Null for a record that is not a text one.
     */
    public function text(): ?string
    {
        if (TypeNameFormat::WellKnown !== $this->format || 'T' !== $this->type || '' === $this->payload) {
            return null;
        }

        // The status byte: the encoding in its high bit, then the length of the language code.
        $status = \ord($this->payload[0]);
        $text = substr($this->payload, 1 + ($status & 0x3F));

        return 0 === ($status & 0x80) ? $text : mb_convert_encoding($text, 'UTF-8', 'UTF-16');
    }

    /**
     * For a URI record or an absolute URI one. Null for any other record.
     */
    public function uri(): ?string
    {
        if (TypeNameFormat::AbsoluteUri === $this->format) {
            return $this->type;
        }
        if (TypeNameFormat::WellKnown !== $this->format || 'U' !== $this->type || '' === $this->payload) {
            return null;
        }

        return (self::URI_PREFIXES[\ord($this->payload[0])] ?? '').substr($this->payload, 1);
    }

    private static function bytes(string $base64): string
    {
        $bytes = base64_decode($base64, true);
        if (false === $bytes) {
            throw new UnexpectedResponseException(\sprintf('The runtime sent bytes of an NFC record that are not base64: "%s".', $base64));
        }

        return $bytes;
    }
}
