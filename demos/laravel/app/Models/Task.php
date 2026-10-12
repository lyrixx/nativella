<?php

namespace App\Models;

use App\Observers\TaskObserver;
use Carbon\CarbonImmutable;
use Database\Factories\TaskFactory;
use Illuminate\Database\Eloquent\Attributes\CollectedBy;
use Illuminate\Database\Eloquent\Attributes\ObservedBy;
use Illuminate\Database\Eloquent\Attributes\Scope;
use Illuminate\Database\Eloquent\Builder;
use Illuminate\Database\Eloquent\Factories\HasFactory;
use Illuminate\Database\Eloquent\Model;

/**
 * @property int                  $id
 * @property string               $title
 * @property CarbonImmutable|null $completed_at
 * @property CarbonImmutable      $created_at
 * @property CarbonImmutable      $updated_at
 */
#[CollectedBy(TaskCollection::class)]
#[ObservedBy(TaskObserver::class)]
final class Task extends Model
{
    /** @use HasFactory<TaskFactory> */
    use HasFactory;

    protected $fillable = ['title'];

    public function isCompleted(): bool
    {
        return null !== $this->completed_at;
    }

    public function complete(): void
    {
        $this->completed_at ??= CarbonImmutable::now();
    }

    public function reopen(): void
    {
        $this->completed_at = null;
    }

    protected function casts(): array
    {
        return [
            'completed_at' => 'immutable_datetime',
        ];
    }

    /**
     * @param Builder<self> $query
     */
    #[Scope]
    protected function open(Builder $query): void
    {
        $query->whereNull('completed_at');
    }

    /**
     * The open tasks first, the newest on top, then the completed ones.
     *
     * @param Builder<self> $query
     */
    #[Scope]
    protected function ordered(Builder $query): void
    {
        $query->orderByRaw('completed_at IS NOT NULL')->orderByDesc('completed_at')->orderByDesc('id');
    }
}
