<?php

namespace App\Tests\Http;

use App\Http\Response;
use PHPUnit\Framework\TestCase;

final class ResponseTest extends TestCase
{
    public function testADownloadNamesItsFileInAsciiAndInUtf8(): void
    {
        $response = Response::download('ssh deploy@prod', 'Déploiement "prod".txt', 'text/plain; charset=UTF-8');

        $this->assertSame('ssh deploy@prod', $response->body);
        $this->assertSame([
            'Content-Disposition' => 'attachment; filename="D_ploiement _prod_.txt"; filename*=UTF-8\'\'D%C3%A9ploiement%20%22prod%22.txt',
            'Content-Type' => 'text/plain; charset=UTF-8',
        ], $response->headers);
    }
}
