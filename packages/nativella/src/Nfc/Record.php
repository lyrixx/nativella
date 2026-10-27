<?php

namespace Nativella\Nfc;

/**
 * A record of the NDEF message of a tag. $type, $id and $payload are bytes:
 * text() and uri() read the records most tags hold.
 */
final readonly class Record
{
    /** The prefixes of the URI records, by their code. */
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
            base64_decode($data['type'], true) ?: '',
            base64_decode($data['id'], true) ?: '',
            base64_decode($data['payload'], true) ?: '',
        );
    }

    /**
     * The text of a text record, null for another one.
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
     * The URI of a URI record, or of an absolute URI one, null for another one.
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
}
