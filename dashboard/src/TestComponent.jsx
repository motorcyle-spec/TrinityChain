import React from 'react';

const TestComponent = () => {
  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-950 via-purple-950 to-slate-950 flex items-center justify-center">
      <div className="bg-white rounded-lg p-8 shadow-xl">
        <h1 className="text-4xl font-bold text-purple-600 mb-4">
          Dashboard Test
        </h1>
        <p className="text-gray-700 mb-2">
          If you can see this styled card, Tailwind CSS is working! ✨
        </p>
        <div className="mt-4 p-4 bg-purple-100 rounded border-2 border-purple-500">
          <p className="text-purple-900 font-semibold">
            Background should be purple gradient
          </p>
          <p className="text-purple-700 text-sm">
            This card should have borders and colors
          </p>
        </div>
        <button className="mt-4 px-6 py-2 bg-purple-600 text-white rounded-lg hover:bg-purple-700 transition-colors">
          Click me if CSS works!
        </button>
      </div>
    </div>
  );
};

export default TestComponent;
