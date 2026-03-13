import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';

export type UseCase = 'business' | 'residential' | 'personal';

interface UseCaseState {
  useCase: UseCase;
  setUseCase: (uc: UseCase) => void;
  hasChosen: boolean;
}

const UseCaseContext = createContext<UseCaseState | null>(null);

const STORAGE_KEY = 'parkhub_usecase';

export function UseCaseProvider({ children }: { children: ReactNode }) {
  const [useCase, setUseCaseState] = useState<UseCase>(() =>
    (localStorage.getItem(STORAGE_KEY) as UseCase) || 'business'
  );
  const [hasChosen, setHasChosen] = useState(() =>
    localStorage.getItem(STORAGE_KEY) !== null
  );

  useEffect(() => {
    const root = document.documentElement;
    root.setAttribute('data-usecase', useCase);
  }, [useCase]);

  function setUseCase(uc: UseCase) {
    setUseCaseState(uc);
    setHasChosen(true);
    localStorage.setItem(STORAGE_KEY, uc);
  }

  return (
    <UseCaseContext.Provider value={{ useCase, setUseCase, hasChosen }}>
      {children}
    </UseCaseContext.Provider>
  );
}

export function useUseCase() {
  const ctx = useContext(UseCaseContext);
  if (!ctx) throw new Error('useUseCase must be used within UseCaseProvider');
  return ctx;
}
