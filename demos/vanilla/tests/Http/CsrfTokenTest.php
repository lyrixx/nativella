<?php

namespace App\Tests\Http;

use App\Http\CsrfToken;
use App\Http\Request;
use PHPUnit\Framework\TestCase;

final class CsrfTokenTest extends TestCase
{
    protected function tearDown(): void
    {
        session_unset();
    }

    public function testTheFormsOfASessionShareTheirToken(): void
    {
        $token = new CsrfToken();

        $this->assertMatchesRegularExpression('/^[0-9a-f]{64}$/', $token->value());
        $this->assertSame($token->value(), new CsrfToken()->value());
    }

    public function testAFormPostedWithTheTokenIsValid(): void
    {
        $token = new CsrfToken();

        $this->assertTrue($token->isValid(new Request('POST', '/snippets', form: [CsrfToken::FIELD => $token->value()])));
    }

    public function testAFormPostedWithoutTheTokenIsNot(): void
    {
        $token = new CsrfToken();
        $token->value();

        $this->assertFalse($token->isValid(new Request('POST', '/snippets')));
        $this->assertFalse($token->isValid(new Request('POST', '/snippets', form: [CsrfToken::FIELD => str_repeat('0', 64)])));
        $this->assertFalse($token->isValid(new Request('POST', '/snippets', form: [CsrfToken::FIELD => ['an array']])));
    }

    public function testASessionWithoutATokenAcceptsNone(): void
    {
        $this->assertFalse(new CsrfToken()->isValid(new Request('POST', '/snippets', form: [CsrfToken::FIELD => ''])));
    }
}
