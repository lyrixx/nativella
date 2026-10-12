<?php

namespace Nativella\Tests;

use Nativella\Client\FakeClient;
use Nativella\Secrets;
use PHPUnit\Framework\TestCase;

final class SecretsTest extends TestCase
{
    public function testSecretsGoToTheKeychain(): void
    {
        $client = new FakeClient();
        $secrets = new Secrets($client);

        $secrets->set('github-token', 'ghp_123');
        $client->respond('secrets.get', ['value' => 'ghp_123']);

        $this->assertSame('ghp_123', $secrets->get('github-token'));
        $this->assertNull($secrets->get('github-token'), 'The fake answers no secret by default.');
        $this->assertSame(['key' => 'github-token', 'value' => 'ghp_123'], $client->calls('secrets.set')[0]['payload']);
    }
}
