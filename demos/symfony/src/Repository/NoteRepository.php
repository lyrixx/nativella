<?php

namespace App\Repository;

use App\Entity\Note;
use App\Note\NoteList;
use App\Note\NoteSummary;
use Doctrine\Bundle\DoctrineBundle\Repository\ServiceEntityRepository;
use Doctrine\Persistence\ManagerRegistry;

/**
 * @extends ServiceEntityRepository<Note>
 */
class NoteRepository extends ServiceEntityRepository
{
    /** The sidebar lists this many notes at most: a search finds the others. */
    public const int LIST_LIMIT = 100;

    public function __construct(ManagerRegistry $registry)
    {
        parent::__construct($registry, Note::class);
    }

    public function add(Note $note): void
    {
        $this->getEntityManager()->persist($note);
    }

    public function remove(Note $note): void
    {
        $this->getEntityManager()->remove($note);
    }

    /**
     * Pinned notes first, then the most recently updated, LIST_LIMIT at most.
     * As SQLite's LIKE, the search ignores the case of ASCII letters only.
     */
    public function findForSidebar(?string $search = null): NoteList
    {
        $query = $this->createQueryBuilder('note')
            ->select(\sprintf('NEW %s(note.id, note.title, note.pinned, note.updatedAt)', NoteSummary::class))
            ->orderBy('note.pinned', 'DESC')
            ->addOrderBy('note.updatedAt', 'DESC')
            ->addOrderBy('note.id', 'DESC')
            // One more than listed tells whether there are more.
            ->setMaxResults(self::LIST_LIMIT + 1);

        if (null !== $search && '' !== trim($search)) {
            $query
                // LIKE has no escape character unless told: "%" and "_" of the search are meant literally.
                ->andWhere("note.title LIKE :search ESCAPE '\\' OR note.body LIKE :search ESCAPE '\\'")
                ->setParameter('search', '%'.addcslashes(trim($search), '%_\\').'%');
        }

        /** @var list<NoteSummary> $summaries */
        $summaries = $query->getQuery()->getResult();

        return new NoteList(\array_slice($summaries, 0, self::LIST_LIMIT), \count($summaries) > self::LIST_LIMIT);
    }

    public function findLatest(): ?Note
    {
        return $this->findOneBy([], ['pinned' => 'DESC', 'updatedAt' => 'DESC']);
    }
}
