import { useQuery, gql } from "@apollo/client";
import { Button } from "./components/catalyst/button";
import { Dropdown, DropdownButton, DropdownItem, DropdownMenu } from './components/catalyst/dropdown';
import { ChevronDownIcon } from '@heroicons/react/16/solid';
import { useEffect, useState } from "react";

// Updated GET_VARIABLES query to accept a manualId as a variable
const GET_VARIABLES = gql`
  query GetVariables($manualId: String!) {
    variables(manualName: $manualId) {
      uuid
      name
    }
  }
`;

const GET_MANUALS = gql`
  query GetManuals {
    manuals {
      id
      name
      description
    }
  }
`;

function DisplayManualsDropdown({ onUpdate }) {
  return (
    <Dropdown>
      <DropdownButton outline>
        Options
        <ChevronDownIcon />
      </DropdownButton>
      <DropdownMenu>
        <DisplayManuals onUpdate={onUpdate} />
      </DropdownMenu>
    </Dropdown>
  );
}

function DisplayManuals({ onUpdate }) {
  const { loading, error, data } = useQuery(GET_MANUALS);

  if (loading) return <p>Loading...</p>;
  if (error) return <p>Error : {error.message}</p>;

  return data.manuals.map(({ id, name }) => (
    <DropdownItem key={id} onClick={() => onUpdate(id)}>{name}</DropdownItem>
  ));
}

function DisplayConstructs({ manualId }) {
  // Updated useQuery to pass manualId as a variable
  const { loading, error, data } = useQuery(GET_VARIABLES, {
    variables: { manualId },
    skip: !manualId, // Skip the query if manualId is null
  });
  
  if (!manualId) return <p>Please select a manual to see the variables.</p>;
  if (loading) return <p>Loading...</p>;
  if (error) return <p>Error : {error.message}</p>;

  return data.variables.map(({ uuid, name }) => (
    <li key={uuid} className="flex justify-between gap-x-6 py-5">
      <div className="flex min-w-0 gap-x-4">
        <div className="min-w-0 flex-auto">
          <p className="text-sm font-semibold leading-6 text-gray-900">{name}</p>
          <p className="mt-1 truncate text-xs leading-5 text-gray-500">
            {uuid}
          </p>
        </div>
      </div>
      <div className="hidden shrink-0 sm:flex sm:flex-col sm:items-end">
        <Button>Sign</Button>
      </div>
    </li>
  ));
}

export function App() {
  const [manualId, setManualId] = useState(null);
  const { data: manualsData, loading: manualsLoading, error: manualsError } = useQuery(GET_MANUALS);

  // Set the first manual's ID as the default state after manuals are fetched
  useEffect(() => {
    if (!manualsLoading && !manualsError && manualsData?.manuals?.length > 0) {
      setManualId(manualsData.manuals[0].id);
    }
  }, [manualsData, manualsLoading, manualsError]);

  if (manualsLoading) return <p>Loading manuals...</p>;
  if (manualsError) return <p>Error loading manuals: {manualsError.message}</p>;

  return (
    <div className="min-h-full">
      <main className="mt-16 ">
        <div className="mx-auto max-w-7xl px-4 pb-12 sm:px-6 lg:px-8">
          <div className="rounded-lg bg-white px-5 py-6 shadow sm:px-6">
            <DisplayManualsDropdown onUpdate={(id) => setManualId(id)} />
            <ul role="list" className="divide-y divide-gray-100">
              {/* Pass manualId to DisplayConstructs */}
              <DisplayConstructs manualId={manualId} />
            </ul>
          </div>
        </div>
      </main>
    </div>
  );
}
