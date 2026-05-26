import type { Chapter, Lesson } from '../types';
import { gettingStarted } from './getting-started';
import { primitiveTypes } from './primitive-types';
import { functions } from './functions';
import { controlFlow } from './control-flow';
import { collections } from './collections';
import { structs } from './structs';
import { enumsMatching } from './enums-matching';
import { errorHandling } from './error-handling';
import { generics } from './generics';
import { traits } from './traits';
import { closures } from './closures';
import { iterators } from './iterators';
import { modules } from './modules';
import { asyncChapter } from './async';
import { stdlib } from './stdlib';

export const CHAPTERS: Chapter[] = [
  gettingStarted,
  primitiveTypes,
  functions,
  controlFlow,
  collections,
  structs,
  enumsMatching,
  errorHandling,
  generics,
  traits,
  closures,
  iterators,
  modules,
  asyncChapter,
  stdlib,
];

export function findLesson(chapterId: string, lessonId: string): Lesson | undefined {
  const chapter = CHAPTERS.find((c) => c.id === chapterId);
  return chapter?.lessons.find((l) => l.id === lessonId);
}

export function getPrevNext(
  chapterId: string,
  lessonId: string,
): { prev: { chapter: string; lesson: string } | null; next: { chapter: string; lesson: string } | null } {
  const flat: { chapter: string; lesson: string }[] = [];
  for (const ch of CHAPTERS) {
    for (const le of ch.lessons) {
      flat.push({ chapter: ch.id, lesson: le.id });
    }
  }
  const idx = flat.findIndex((e) => e.chapter === chapterId && e.lesson === lessonId);
  return {
    prev: idx > 0 ? flat[idx - 1] : null,
    next: idx < flat.length - 1 ? flat[idx + 1] : null,
  };
}

export function getChapter(chapterId: string): Chapter | undefined {
  return CHAPTERS.find((c) => c.id === chapterId);
}
