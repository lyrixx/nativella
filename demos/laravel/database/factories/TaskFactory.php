<?php

namespace Database\Factories;

use App\Models\Task;
use Carbon\CarbonImmutable;
use Illuminate\Database\Eloquent\Factories\Factory;

/**
 * @extends Factory<Task>
 */
final class TaskFactory extends Factory
{
    protected $model = Task::class;

    public function definition(): array
    {
        return [
            'title' => fake()->sentence(3),
        ];
    }

    public function completed(): self
    {
        return $this->state(['completed_at' => CarbonImmutable::now()]);
    }
}
